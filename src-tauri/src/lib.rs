//! Сборка Tauri-приложения InfinityConnect (Windows).
//!
//! Слои бэкенда (по мере роста проекта): api / subscription / engine / tunnel /
//! sidecar / ping / routing / store / device — см. ARCHITECTURE.md. На Фазе 0
//! подключены только `commands` (мост invoke) и `state` (эмит состояния),
//! системный трей и плагин автозапуска.

mod api;
mod commands;
mod device;
mod engine;
mod error;
mod state;
mod store;
mod subscription;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::MacosLauncher;

use crate::api::ApiClient;
use crate::state::{emit_state, TunnelState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Автозапуск с ОС (в трее). Аргументы запуска — пусто.
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        // Общий HTTP-клиент к серверу (discovery/токены восстанавливаются из кэша).
        .manage(ApiClient::new())
        .setup(|app| {
            build_tray(app.handle())?;
            // Эмитим стартовое состояние туннеля — мост emit end-to-end.
            emit_state(app.handle(), TunnelState::Disconnected);
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска InfinityConnect");
}

/// Системный трей: статус + пункты «Показать» и «Выход».
/// connect/disconnect из трея добавим на Фазе 4 (когда появится сам туннель).
fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Показать", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Infinity Connect")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
