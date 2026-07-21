//! Единый источник состояния туннеля для UI (аналог Android VpnStateHolder).
//! Состояние и статистика эмитятся во фронт через Tauri-события; фронт слушает
//! их (см. `src/api/commands.ts`).

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter};

/// Состояние туннеля (зеркало TunnelStateEvent на фронте).
#[derive(Clone, Serialize)]
pub enum TunnelState {
    Disconnected,
    Connecting,
    /// Подключено к серверу (remark — для отображения).
    Connected(String),
    Error(String),
}

/// Канал событий состояния туннеля.
pub const TUNNEL_STATE_EVENT: &str = "tunnel://state";
/// Канал событий статистики трафика.
pub const TUNNEL_STATS_EVENT: &str = "tunnel://stats";

/// Эмитит текущее состояние туннеля во фронт.
pub fn emit_state(app: &AppHandle, state: TunnelState) {
    let payload = match state {
        TunnelState::Disconnected => json!({"status": "disconnected"}),
        TunnelState::Connecting => json!({"status": "connecting"}),
        TunnelState::Connected(remark) => json!({"status": "connected", "message": remark}),
        TunnelState::Error(m) => json!({"status": "error", "message": m}),
    };
    if let Err(e) = app.emit(TUNNEL_STATE_EVENT, payload) {
        eprintln!("emit tunnel state failed: {e}");
    }
}

/// Эмитит статистику трафика: суммарные байты + мгновенная скорость (байт/с).
pub fn emit_stats(app: &AppHandle, up_bytes: u64, down_bytes: u64, up_speed: u64, down_speed: u64) {
    let payload = json!({
        "upBytes": up_bytes,
        "downBytes": down_bytes,
        "upSpeed": up_speed,
        "downSpeed": down_speed,
    });
    if let Err(e) = app.emit(TUNNEL_STATS_EVENT, payload) {
        eprintln!("emit tunnel stats failed: {e}");
    }
}
