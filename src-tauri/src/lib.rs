//! Сборка Tauri-приложения InfinityConnect (Windows).
//!
//! Слои бэкенда (по мере роста проекта): api / subscription / engine / tunnel /
//! sidecar / ping / routing / store / device — см. ARCHITECTURE.md. На Фазе 0
//! подключены только `commands` (мост invoke) и `state` (эмит состояния),
//! системный трей и плагин автозапуска.

mod api;
mod apps;
mod commands;
mod connection;
mod device;
mod elevation;
mod engine;
mod error;
mod ping;
mod routing;
mod sidecar;
mod state;
mod store;
mod subscription;
mod tunnel;

pub use elevation::{is_elevated, relaunch_elevated};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::MacosLauncher;

use crate::api::ApiClient;
use crate::ping::Pinger;
use crate::state::{emit_state, TunnelState};
use crate::tunnel::TunnelManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance ДОЛЖЕН быть первым плагином: повторный запуск (в т.ч.
        // по deep-link infinityconnect://…) передаёт argv сюда и завершается.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Просто фокусируем окно. Deep-link второго экземпляра доставит сам
            // deep-link плагин через on_open_url (feature "deep-link" у
            // single-instance), поэтому argv тут парсить не нужно.
            show_main(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        // Автозапуск с ОС (в трее). Аргументы запуска — пусто.
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        // Авто-обновление (проверка/скачивание/установка подписанных релизов).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // Общий HTTP-клиент к серверу (discovery/токены восстанавливаются из кэша).
        .manage(ApiClient::new())
        // Слот для последнего результата desktop-авторизации (гонка emit/listen).
        .manage(AuthResultSlot::default())
        .setup(|app| {
            build_tray(app.handle())?;

            // Регистрируем схему infinityconnect:// в реестре текущего пользователя
            // (runtime-регистрация надёжнее на Windows: работает и без инсталлера).
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register("infinityconnect");
                // Deep-link, пришедший в ЭТОТ экземпляр (первый запуск по ссылке).
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        handle_deep_link(handle.clone(), url.to_string());
                    }
                });
            }

            // Каталог с xray.exe/wintun.dll/geo-файлами (bundled resources).
            // В dev — src-tauri/binaries; в проде — resource_dir/binaries.
            let bin_dir = resolve_binaries_dir(app.handle());
            app.manage(TunnelManager::new(bin_dir.clone()));
            app.manage(Pinger::new(bin_dir.clone()));
            // Каталог логов ядер (там же лежат *_stderr.log) — для экрана логов.
            app.manage(commands::LogsDir(bin_dir));

            // Эмитим стартовое состояние туннеля — мост emit end-to-end.
            emit_state(app.handle(), TunnelState::Disconnected);

            // Автообновление подписок: первый прогон через ~10с после старта,
            // далее раз в 12 часов. Обновляет зашифрованный кэш тел подписок.
            {
                use tauri::Manager;
                let api = app.state::<ApiClient>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    loop {
                        let _ = api.refresh_subscriptions().await;
                        tokio::time::sleep(std::time::Duration::from_secs(12 * 60 * 60)).await;
                    }
                });
            }
            Ok(())
        })
        // Крестик прячет окно в трей, а не завершает приложение (VPN продолжает
        // работать в фоне). Выход — через пункт трея «Выход».
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::discover,
            commands::login,
            commands::logout,
            commands::is_authorized,
            commands::user_info,
            commands::subscription_info,
            commands::support_url,
            commands::site_auth_url,
            commands::take_auth_result,
            commands::open_url,
            commands::keys,
            commands::refresh_subscriptions,
            commands::key_servers,
            commands::connect,
            commands::disconnect,
            commands::tunnel_status,
            commands::is_autostart_enabled,
            commands::set_autostart,
            commands::ping_server,
            commands::get_ping_settings,
            commands::set_ping_settings,
            commands::get_routing_settings,
            commands::set_routing_settings,
            commands::list_installed_apps,
            commands::read_core_logs,
            commands::clear_core_logs,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска InfinityConnect");
}

/// Обрабатывает deep-link `infinityconnect://auth?code=…`: меняет одноразовый
/// код на токены через API и эмитит фронту `auth://result` (ok | текст ошибки).
fn handle_deep_link(app: tauri::AppHandle, url: String) {
    // Интересует только auth-колбэк; прочие ссылки молча игнорируем.
    let Some(code) = parse_auth_code(&url) else { return };
    show_main(&app);
    tauri::async_runtime::spawn(async move {
        use tauri::Emitter;
        let api = app.state::<ApiClient>().inner().clone();
        let payload = match api.exchange_auth_code(&code).await {
            Ok(()) => serde_json::json!({ "ok": true }),
            Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
        };
        // Сохраняем результат ДО эмита: если фронт ещё не подписался (окно только
        // что показано по deep-link), он заберёт результат командой при монтировании.
        if let Some(slot) = app.try_state::<AuthResultSlot>() {
            *slot.0.lock().unwrap() = Some(payload.clone());
        }
        let _ = app.emit("auth://result", payload);
    });
}

/// Последний результат desktop-авторизации — на случай гонки emit/listen
/// (deep-link поднял окно, фронт ещё не успел повесить слушатель).
#[derive(Default)]
pub struct AuthResultSlot(pub std::sync::Mutex<Option<serde_json::Value>>);

/// Извлекает `code` из `infinityconnect://auth?code=…`. None — не auth-ссылка.
fn parse_auth_code(url: &str) -> Option<String> {
    let rest = url.strip_prefix("infinityconnect://")?;
    // Хост-часть может быть "auth" или "auth/" (браузеры добавляют слэш).
    let (host, query) = rest.split_once('?')?;
    if host.trim_end_matches('/') != "auth" {
        return None;
    }
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("code="))
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty() && c.len() <= 256 && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
}

#[cfg(test)]
mod deep_link_tests {
    use super::parse_auth_code;

    #[test]
    fn parses_auth_code() {
        assert_eq!(parse_auth_code("infinityconnect://auth?code=abc-123_X").as_deref(), Some("abc-123_X"));
        assert_eq!(parse_auth_code("infinityconnect://auth/?code=zzz&x=1").as_deref(), Some("zzz"));
    }

    #[test]
    fn rejects_non_auth_and_bad_codes() {
        assert_eq!(parse_auth_code("infinityconnect://other?code=abc"), None);
        assert_eq!(parse_auth_code("infinityconnect://auth?code="), None);
        assert_eq!(parse_auth_code("infinityconnect://auth?code=with space"), None);
        assert_eq!(parse_auth_code("https://evil/auth?code=abc"), None);
    }
}

/// Каталог с ядром и wintun.dll. В проде — рядом с ресурсами приложения; в dev —
/// `src-tauri/binaries`. Ядро само ищет wintun.dll в своём cwd (мы ставим cwd в этот каталог).
fn resolve_binaries_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    use std::path::PathBuf;
    let mut tried: Vec<PathBuf> = Vec::new();

    // 1. Прод (NSIS/MSI): ресурсы бандла (resources: ["binaries/*"]).
    if let Ok(res) = app.path().resource_dir() {
        let candidate = res.join("binaries");
        tried.push(candidate.clone());
        if candidate.join("xray.exe").exists() {
            return candidate;
        }
    }

    // 2. Фолбэк для кастомного установщика: binaries/ рядом с самим exe.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("binaries");
            tried.push(candidate.clone());
            if candidate.join("xray.exe").exists() {
                return candidate;
            }
        }
    }

    // 3. Dev-фолбэк: каталог исходников.
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
    if dev.join("xray.exe").exists() {
        return dev;
    }
    tried.push(dev.clone());

    // Ничего не нашли — пишем диагностику рядом с exe и в %TEMP%.
    let msg = format!(
        "InfinityConnect: не найден каталог ядер (xray.exe). Проверены пути:\n{}\n",
        tried
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let _ = std::fs::write(std::env::temp_dir().join("infinity-binaries.log"), &msg);
    dev
}

/// Системный трей: показать окно, отключить VPN, выход. Клик по иконке — показать.
fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Показать окно", true, None::<&str>)?;
    let disconnect = MenuItem::with_id(app, "disconnect", "Отключить VPN", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &disconnect, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Infinity Connect")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "disconnect" => {
                // Гасим туннель из фонового рантайма (команда async).
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(tunnel) = app.try_state::<TunnelManager>() {
                        tunnel.disconnect(&app).await;
                    }
                });
            }
            "quit" => {
                // Перед выходом гасим туннель (снять маршруты/адаптер).
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(tunnel) = app.try_state::<TunnelManager>() {
                        tunnel.disconnect(&app).await;
                    }
                    app.exit(0);
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Левый клик по иконке — показать окно.
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Показывает и фокусирует главное окно.
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
