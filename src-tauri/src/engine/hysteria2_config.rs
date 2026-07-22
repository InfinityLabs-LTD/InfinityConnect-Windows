//! Сборка JSON-конфига клиента Hysteria2 из `Hysteria2Config`
//! (аналог Android `Hysteria2ConfigBuilder.kt`, но с tun-режимом для Windows).
//!
//! **Отличие от Android:** там TUN-fd передавался ядру через gomobile. На Windows
//! официальный hysteria.exe сам поднимает wintun-адаптер через секцию `tun`
//! (подтверждено: ядро парсит секцию и создаёт адаптер). Формат клиента —
//! github.com/apernet/hysteria (server/auth/tls/obfs/bandwidth) + tun + trafficStats.

use serde_json::{json, Value};

use super::Hysteria2Config;

/// Имя wintun-адаптера Hysteria (отличается от Xray-адаптера).
pub const TUN_NAME: &str = "InfinityHy";
pub const TUN_ADDRESS: &str = "10.20.0.2/30";
/// Локальный порт HTTP API статистики (trafficStats).
pub const STATS_API_PORT: u16 = 10086;

/// Строит конфиг клиента Hysteria2 в tun-режиме.
pub fn build(config: &Hysteria2Config, mtu: u32) -> String {
    let mut root = json!({
        "server": format!("{}:{}", host_for_server(&config.address), config.port),
        "auth": config.auth,
        "tls": tls_block(config),
        "tun": {
            "name": TUN_NAME,
            "mtu": mtu,
            "timeout": "5m",
            "address": {"ipv4": TUN_ADDRESS},
            // Весь трафик в туннель; приватные сети ядро обходит само по route.
            "route": {
                "ipv4": ["0.0.0.0/0"],
                "ipv4Exclude": ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "127.0.0.0/8"]
            }
        },
        // HTTP API для чтения статистики трафика (sidecar опрашивает).
        "trafficStats": {"listen": format!("127.0.0.1:{STATS_API_PORT}")}
    });
    let obj = root.as_object_mut().unwrap();

    // obfs (salamander) — если задан пароль.
    if let Some(pwd) = config.obfs_password.as_deref().filter(|s| !s.is_empty()) {
        obj.insert(
            "obfs".into(),
            json!({"type": "salamander", "salamander": {"password": pwd}}),
        );
    }

    // bandwidth — если заданы лимиты.
    if config.up_mbps.is_some() || config.down_mbps.is_some() {
        let mut bw = json!({});
        let b = bw.as_object_mut().unwrap();
        if let Some(up) = config.up_mbps {
            b.insert("up".into(), json!(format!("{up} mbps")));
        }
        if let Some(down) = config.down_mbps {
            b.insert("down".into(), json!(format!("{down} mbps")));
        }
        obj.insert("bandwidth".into(), bw);
    }

    root.to_string()
}

/// Гибрид: hysteria слушает локальный SOCKS5 вместо TUN (TUN поднимает sing-box).
/// Тот же server/auth/tls/obfs/bandwidth, но выход — socks5 на 127.0.0.1:port.
pub fn build_socks_proxy(config: &Hysteria2Config, socks_port: u16) -> String {
    let mut root = json!({
        "server": format!("{}:{}", host_for_server(&config.address), config.port),
        "auth": config.auth,
        "tls": tls_block(config),
        "socks5": {"listen": format!("127.0.0.1:{socks_port}")},
        "trafficStats": {"listen": format!("127.0.0.1:{STATS_API_PORT}")}
    });
    let obj = root.as_object_mut().unwrap();
    if let Some(pwd) = config.obfs_password.as_deref().filter(|s| !s.is_empty()) {
        obj.insert("obfs".into(), json!({"type": "salamander", "salamander": {"password": pwd}}));
    }
    if config.up_mbps.is_some() || config.down_mbps.is_some() {
        let mut bw = json!({});
        let b = bw.as_object_mut().unwrap();
        if let Some(up) = config.up_mbps {
            b.insert("up".into(), json!(format!("{up} mbps")));
        }
        if let Some(down) = config.down_mbps {
            b.insert("down".into(), json!(format!("{down} mbps")));
        }
        obj.insert("bandwidth".into(), bw);
    }
    root.to_string()
}

fn tls_block(config: &Hysteria2Config) -> Value {
    let mut tls = json!({"insecure": config.insecure});
    if let Some(sni) = config.sni.as_deref().filter(|s| !s.is_empty()) {
        tls.as_object_mut().unwrap().insert("sni".into(), json!(sni));
    }
    tls
}

/// IPv6-хост берём в квадратные скобки для "host:port".
fn host_for_server(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Hysteria2Config {
        Hysteria2Config {
            remark: "Hy2".into(),
            address: "example.com".into(),
            port: 443,
            auth: "secret-auth".into(),
            sni: Some("example.com".into()),
            insecure: false,
            obfs_password: Some("obfs-pass".into()),
            up_mbps: Some(50),
            down_mbps: Some(200),
        }
    }

    #[test]
    fn builds_expected_hysteria_shape() {
        let json = build(&sample(), crate::engine::xray_config::DEFAULT_MTU);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["server"], "example.com:443");
        assert_eq!(v["auth"], "secret-auth");
        assert_eq!(v["tls"]["sni"], "example.com");
        assert_eq!(v["tls"]["insecure"], false);
        assert_eq!(v["obfs"]["type"], "salamander");
        assert_eq!(v["obfs"]["salamander"]["password"], "obfs-pass");
        assert_eq!(v["bandwidth"]["up"], "50 mbps");
        assert_eq!(v["bandwidth"]["down"], "200 mbps");
        // tun-режим для Windows.
        assert_eq!(v["tun"]["name"], TUN_NAME);
        assert_eq!(v["trafficStats"]["listen"], format!("127.0.0.1:{STATS_API_PORT}"));
    }

    #[test]
    fn ipv6_host_is_bracketed() {
        let mut c = sample();
        c.address = "2001:db8::1".into();
        let json = build(&c, crate::engine::xray_config::DEFAULT_MTU);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["server"], "[2001:db8::1]:443");
    }
}
