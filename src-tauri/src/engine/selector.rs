//! Выбор ядра по профилю (аналог Android `EngineSelector`).
//! Vless/RawXray → Xray, Hysteria2 → Hysteria. Возвращает всё, что нужно
//! оркестратору туннеля: тип ядра, готовый JSON-конфиг и имя wintun-адаптера.

use crate::routing::RoutingSettings;

use super::{hysteria2_config, xray_config, EngineConfig};

/// Какое ядро запускать.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreKind {
    Xray,
    Hysteria,
}

/// План запуска: ядро + конфиг + имя адаптера + порт stats API.
pub struct CorePlan {
    pub kind: CoreKind,
    pub config_json: String,
    pub tun_name: &'static str,
    pub stats_port: u16,
    pub remark: String,
    /// Адрес VPN-сервера (для kill-switch: разрешить трафик до него).
    pub server_ip: String,
}

/// Строит план запуска для профиля. `mtu` — MTU tun-интерфейса. `routing` —
/// пользовательские правила по сайтам (применяются только к VLESS; RawXray несёт
/// свой серверный routing, Hysteria2 доменные правила из UI не применяет).
pub fn select(config: &EngineConfig, mtu: u32, routing: &RoutingSettings) -> CorePlan {
    let remark = config.remark().to_string();
    let server_ip = config.address().to_string();
    match config {
        EngineConfig::Vless(v) => CorePlan {
            kind: CoreKind::Xray,
            config_json: xray_config::build_vless(v, mtu, routing),
            tun_name: xray_config::TUN_NAME,
            stats_port: xray_config::STATS_API_PORT,
            remark,
            server_ip,
        },
        EngineConfig::RawXray(r) => CorePlan {
            kind: CoreKind::Xray,
            config_json: xray_config::build_raw(r, mtu),
            tun_name: xray_config::TUN_NAME,
            stats_port: xray_config::STATS_API_PORT,
            remark,
            server_ip,
        },
        EngineConfig::Hysteria2(h) => CorePlan {
            kind: CoreKind::Hysteria,
            config_json: hysteria2_config::build(h, mtu),
            tun_name: hysteria2_config::TUN_NAME,
            stats_port: hysteria2_config::STATS_API_PORT,
            remark,
            server_ip,
        },
    }
}
