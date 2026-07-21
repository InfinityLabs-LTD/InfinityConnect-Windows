//! Единый источник состояния туннеля для UI (аналог Android VpnStateHolder).
//! Состояние эмитится во фронт через Tauri-события `tunnel://state`; фронт
//! слушает их (см. `src/api/commands.ts`). На Фазе 0 — только типы и эмит
//! стартового `disconnected`.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Событие состояния туннеля (зеркало TunnelStateEvent на фронте).
// Фаза 0: варианты Connecting/Connected/Error ещё не эмитятся (нет туннеля) —
// появятся на Фазе 2. Заглушаем dead_code, чтобы каркас собирался без warning.
#[allow(dead_code)]
#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase", tag = "status", content = "message")]
pub enum TunnelState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Имя канала событий, на который подписан фронт.
pub const TUNNEL_STATE_EVENT: &str = "tunnel://state";

/// Эмитит текущее состояние туннеля во фронт.
pub fn emit_state(app: &AppHandle, state: TunnelState) {
    // Ошибку эмита логируем, но не роняем приложение.
    if let Err(e) = app.emit(TUNNEL_STATE_EVENT, wire(state)) {
        eprintln!("emit tunnel state failed: {e}");
    }
}

/// Приводит enum к форме {status, message?}, которую ждёт фронт.
fn wire(state: TunnelState) -> serde_json::Value {
    match state {
        TunnelState::Disconnected => serde_json::json!({ "status": "disconnected" }),
        TunnelState::Connecting => serde_json::json!({ "status": "connecting" }),
        TunnelState::Connected => serde_json::json!({ "status": "connected" }),
        TunnelState::Error(m) => serde_json::json!({ "status": "error", "message": m }),
    }
}
