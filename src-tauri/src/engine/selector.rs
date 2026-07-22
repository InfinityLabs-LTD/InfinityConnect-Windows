//! Выбор ядра-прокси по профилю (гибридная архитектура, как у Happ).
//! sing-box ВСЕГДА поднимает TUN и маршрутизирует по процессам; проксирование —
//! внешнее ядро (xray для VLESS/RawXray, hysteria для Hysteria2) как локальный
//! SOCKS. Это даёт per-app split-tunnel и не ломает XHTTP (его держит xray).

use crate::routing::RoutingSettings;

use super::{hysteria2_config, singbox_config, xray_config, EngineConfig};

/// Какое ядро-прокси запускать (кроме sing-box, который поднимает TUN всегда).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreKind {
    Xray,
    Hysteria,
}

/// План запуска гибрида: sing-box (TUN+routing) + ядро-прокси (socks).
pub struct HybridPlan {
    /// Ядро-прокси.
    pub proxy_kind: CoreKind,
    /// Конфиг ядра-прокси (слушает локальный SOCKS `proxy_socks_port`).
    pub proxy_config_json: String,
    /// Конфиг sing-box (TUN + route по процессам → SOCKS ядра-прокси).
    pub singbox_config_json: String,
    /// Порт локального SOCKS, на котором слушает ядро-прокси.
    pub proxy_socks_port: u16,
    /// Имя TUN-адаптера (создаёт sing-box).
    pub tun_name: &'static str,
    /// Порт stats API ядра-прокси (для счётчика трафика).
    pub stats_port: u16,
    pub remark: String,
    /// Адрес VPN-сервера (для kill-switch + host-bypass-маршрут).
    pub server_ip: String,
}

/// Строит гибридный план. `mtu` — MTU TUN. `routing` — сайты (→ xray routing) и
/// приложения (→ sing-box route по процессам, 3 режима).
pub fn select(config: &EngineConfig, mtu: u32, routing: &RoutingSettings) -> HybridPlan {
    let remark = config.remark().to_string();
    let server_ip = config.address().to_string();
    let socks_port = singbox_config::PROXY_SOCKS_PORT;
    let singbox_config_json = singbox_config::build(mtu, routing);

    match config {
        EngineConfig::Vless(v) => HybridPlan {
            proxy_kind: CoreKind::Xray,
            proxy_config_json: xray_config::build_socks_proxy(v, socks_port),
            singbox_config_json,
            proxy_socks_port: socks_port,
            tun_name: singbox_config::TUN_NAME,
            stats_port: xray_config::STATS_API_PORT,
            remark,
            server_ip,
        },
        EngineConfig::RawXray(r) => HybridPlan {
            proxy_kind: CoreKind::Xray,
            proxy_config_json: xray_config::build_socks_proxy_raw(r, socks_port),
            singbox_config_json,
            proxy_socks_port: socks_port,
            tun_name: singbox_config::TUN_NAME,
            stats_port: xray_config::STATS_API_PORT,
            remark,
            server_ip,
        },
        EngineConfig::Hysteria2(h) => HybridPlan {
            proxy_kind: CoreKind::Hysteria,
            proxy_config_json: hysteria2_config::build_socks_proxy(h, socks_port),
            singbox_config_json,
            proxy_socks_port: socks_port,
            tun_name: singbox_config::TUN_NAME,
            stats_port: hysteria2_config::STATS_API_PORT,
            remark,
            server_ip,
        },
    }
}
