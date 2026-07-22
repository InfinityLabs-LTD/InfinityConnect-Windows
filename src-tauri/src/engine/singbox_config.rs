//! Генерация конфига sing-box для гибридной архитектуры (как у Happ).
//!
//! **Роли:** sing-box поднимает TUN и маршрутизирует трафик ПО ПРОЦЕССАМ, а само
//! проксирование делает внешнее ядро (xray/hysteria) как локальный socks-прокси.
//! Это даёт настоящий per-app split-tunnel (Xray+wintun так не умеет — нет
//! process-matching) и не ломает XHTTP (его держит xray, не sing-box).
//!
//! **Три режима** (аналог экрана Happ «Настройки прокси для приложений»):
//!  - `Off`   — весь трафик в VPN (`final: proxy`).
//!  - `Allow` — через VPN ТОЛЬКО выбранные (`process_name → proxy`, `final: direct`).
//!  - `Disallow` — через VPN всё, КРОМЕ выбранных (`process_name → direct`, `final: proxy`).
//!
//! **Фикс Discord:** матчим по `process_name` (имя exe, напр. `Discord.exe`), а НЕ по
//! пути — путь у Discord/Codex меняется при обновлении (`app-1.0.xxxx`), имя нет.

use serde_json::{json, Value};

use crate::routing::{AppRoutingMode, RoutingSettings};

/// Имя TUN-адаптера sing-box.
pub const TUN_NAME: &str = "InfinityTun";
/// Адрес TUN-сети (gateway .1/30, ядру не нужен — sing-box сам).
pub const TUN_ADDRESS: &str = "10.10.0.1/30";
/// Локальный socks-порт, на котором слушает xray-прокси. НЕ 10808 — его занимает Happ.
pub const PROXY_SOCKS_PORT: u16 = 11080;

/// Имена ядер, которые НЕЛЬЗЯ заворачивать в прокси (иначе петля).
const CORE_PROCESS_NAMES: &[&str] = &["xray.exe", "sing-box.exe", "hysteria.exe"];

/// Строит конфиг sing-box: TUN + socks-outbound(→xray) + direct + route по процессам.
/// `mtu` — MTU TUN; `routing` — настройки split-tunnel (режим + список приложений).
pub fn build(mtu: u32, routing: &RoutingSettings) -> String {
    json!({
        "log": {"level": "warn", "timestamp": true},
        "dns": {
            // Резолвим через прокси (анти-утечка DNS), fallback — direct.
            "servers": [
                {"tag": "dns-proxy", "address": "1.1.1.1", "detour": "proxy"},
                {"tag": "dns-direct", "address": "8.8.8.8", "detour": "direct"}
            ],
            "final": "dns-proxy",
            "strategy": "prefer_ipv4"
        },
        "inbounds": [{
            "type": "tun",
            "tag": "tun-in",
            "interface_name": TUN_NAME,
            "address": [TUN_ADDRESS],
            "mtu": mtu,
            "auto_route": true,
            // strict_route: принудительно заворачивает трафик в TUN и рвёт уже
            // открытые соединения мимо туннеля → приложения (Chrome/QUIC)
            // переподключаются САМИ, без ручного перезахода (паритет с Happ).
            "strict_route": true,
            "stack": "mixed"
        }],
        "outbounds": [
            {
                "type": "socks",
                "tag": "proxy",
                "server": "127.0.0.1",
                "server_port": PROXY_SOCKS_PORT,
                "version": "5"
            },
            {"type": "direct", "tag": "direct"}
        ],
        "route": build_route(routing)
    })
    .to_string()
}

/// Секция route: правила по процессам согласно режиму + служебные.
fn build_route(routing: &RoutingSettings) -> Value {
    let mut rules = vec![
        // Перехват DNS и sniff протокола (нужно для доменных правил и DNS-детура).
        json!({"action": "sniff"}),
        json!({"action": "hijack-dns", "protocol": "dns"}),
        // Ядра-прокси — всегда direct (иначе трафик xray к серверу зациклится в tun).
        json!({"process_name": CORE_PROCESS_NAMES, "outbound": "direct"}),
    ];

    // Список выбранных приложений (имена exe). Пустые/битые отбрасываем.
    let apps: Vec<String> = routing
        .apps
        .iter()
        .map(|a| normalize_exe_name(a))
        .filter(|a| !a.is_empty())
        .collect();

    let (final_outbound, app_rule) = match routing.app_mode {
        // Весь трафик в VPN.
        AppRoutingMode::Off => ("proxy", None),
        // Через VPN только выбранные; остальное — direct.
        AppRoutingMode::Allow if !apps.is_empty() => (
            "direct",
            Some(json!({"process_name": apps, "outbound": "proxy"})),
        ),
        AppRoutingMode::Allow => ("proxy", None), // список пуст → как Off
        // Через VPN всё, кроме выбранных.
        AppRoutingMode::Disallow if !apps.is_empty() => (
            "proxy",
            Some(json!({"process_name": apps, "outbound": "direct"})),
        ),
        AppRoutingMode::Disallow => ("proxy", None),
    };

    if let Some(rule) = app_rule {
        rules.push(rule);
    }

    json!({
        "auto_detect_interface": true,
        "final": final_outbound,
        "rules": rules
    })
}

/// Приводит запись приложения к имени exe: из полного пути берёт basename, гарантирует
/// расширение `.exe`. Так список переживает обновления с меняющимся путём (Discord).
fn normalize_exe_name(entry: &str) -> String {
    let s = entry.trim();
    if s.is_empty() {
        return String::new();
    }
    // Берём часть после последнего слэша/бэкслэша.
    let name = s
        .rsplit(|c| c == '\\' || c == '/')
        .next()
        .unwrap_or(s)
        .trim();
    let lower = name.to_lowercase();
    if lower.ends_with(".exe") {
        name.to_string()
    } else {
        format!("{name}.exe")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_mode_sends_all_to_proxy() {
        let r = RoutingSettings::default();
        let v: Value = serde_json::from_str(&build(1500, &r)).unwrap();
        assert_eq!(v["route"]["final"], "proxy");
    }

    #[test]
    fn allow_mode_routes_selected_to_proxy() {
        let r = RoutingSettings {
            app_mode: AppRoutingMode::Allow,
            apps: vec!["Discord.exe".into(), "chrome".into()],
            ..Default::default()
        };
        let v: Value = serde_json::from_str(&build(1500, &r)).unwrap();
        assert_eq!(v["route"]["final"], "direct");
        // Последнее правило — process_name выбранных → proxy.
        let rules = v["route"]["rules"].as_array().unwrap();
        let last = rules.last().unwrap();
        assert_eq!(last["outbound"], "proxy");
        assert_eq!(last["process_name"][0], "Discord.exe");
        assert_eq!(last["process_name"][1], "chrome.exe"); // добавлено .exe
    }

    #[test]
    fn disallow_mode_routes_selected_to_direct() {
        let r = RoutingSettings {
            app_mode: AppRoutingMode::Disallow,
            apps: vec![r"C:\Users\x\AppData\Local\Discord\app-1.0.9245\Discord.exe".into()],
            ..Default::default()
        };
        let v: Value = serde_json::from_str(&build(1500, &r)).unwrap();
        assert_eq!(v["route"]["final"], "proxy");
        let rules = v["route"]["rules"].as_array().unwrap();
        let last = rules.last().unwrap();
        assert_eq!(last["outbound"], "direct");
        // Из полного пути извлечено только имя — переживёт обновление Discord.
        assert_eq!(last["process_name"][0], "Discord.exe");
    }

    #[test]
    fn cores_always_direct() {
        let r = RoutingSettings::default();
        let v: Value = serde_json::from_str(&build(1500, &r)).unwrap();
        let rules = v["route"]["rules"].as_array().unwrap();
        assert!(rules.iter().any(|r| r["process_name"]
            .as_array()
            .map(|a| a.iter().any(|n| n == "xray.exe"))
            .unwrap_or(false)
            && r["outbound"] == "direct"));
    }
}
