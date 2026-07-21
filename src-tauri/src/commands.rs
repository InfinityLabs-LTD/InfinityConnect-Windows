//! Мост фронт↔бэк: `#[tauri::command]`-функции, вызываемые из фронта через
//! invoke(). На Фазе 0 — только тестовая команда `ping`. Дальше сюда придут
//! login/discovery/connect/disconnect/ping-сервера/настройки маршрутизации.

/// Тестовая команда Фазы 0: подтверждает, что мост invoke работает end-to-end.
#[tauri::command]
pub fn ping(name: String) -> String {
    format!("pong: привет, {name} (из Rust-бэкенда)")
}
