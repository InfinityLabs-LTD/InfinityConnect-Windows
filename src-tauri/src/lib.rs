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
        .setup(|app| {
            build_tray(app.handle())?;

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
            commands::keys,
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

/// Каталог с ядром и wintun.dll. В проде — рядом с ресурсами приложения; в dev —
/// `src-tauri/binaries`. Ядро само ищет wintun.dll в своём cwd (мы ставим cwd в этот каталог).
fn resolve_binaries_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    // Прод: ресурсы бандла (resources: ["binaries/*"]).
    if let Ok(res) = app.path().resource_dir() {
        let candidate = res.join("binaries");
        if candidate.join("xray.exe").exists() {
            return candidate;
        }
    }
    // Dev-фолбэк: каталог исходников.
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
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
