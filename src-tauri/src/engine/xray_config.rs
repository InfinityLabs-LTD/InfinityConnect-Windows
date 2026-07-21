//! Сборка JSON-конфигурации Xray-core из `EngineConfig`
//! (аналог Android `XrayConfigBuilder.kt`).
//!
//! **Отличие от Android:** там TUN — инбаунд форка (fd через env). На Windows
//! официальный xray.exe сам поднимает wintun-адаптер по имени через tun-инбаунд
//! (`protocol:"tun"`), поэтому отдельный tun2socks не нужен. Остальное
//! (outbounds/streamSettings/routing/dns) — тот же JSON, что на Android.
//!
//! Добавлен stats API (dokodemo-door + api-сервис) — иначе счётчик трафика пуст.

use serde_json::{json, Value};

use super::{RawXrayConfig, Security, Transport, VlessConfig};

/// Имя wintun-адаптера, создаваемого ядром.
pub const TUN_NAME: &str = "InfinityTun";
pub const TUN_ADDRESS: &str = "10.10.0.2";
pub const DEFAULT_MTU: u32 = 1500;
/// Локальный порт stats API (dokodemo-door → StatsService).
pub const STATS_API_PORT: u16 = 10085;

/// Строит конфиг для TUN-режима из VLESS-профиля: tun-инбаунд + stats API,
/// outbound vless + freedom/block, DNS и routing (весь трафик в proxy,
/// приватные сети — direct).
pub fn build_vless(config: &VlessConfig, mtu: u32) -> String {
    let root = json!({
        "log": {"loglevel": "warning"},
        "stats": {},
        "api": {"tag": "api", "services": ["StatsService"]},
        "policy": {"system": {"statsOutboundUplink": true, "statsOutboundDownlink": true}},
        "dns": {"servers": ["1.1.1.1", "8.8.8.8"]},
        "inbounds": [tun_inbound(mtu), stats_inbound()],
        "outbounds": [
            vless_outbound(config),
            {"tag": "direct", "protocol": "freedom"},
            {"tag": "block", "protocol": "blackhole"}
        ],
        "routing": default_routing(),
    });
    root.to_string()
}

/// Пробрасывает готовый Xray-конфиг из подписки (RawXray) в ядро, подменяя
/// inbounds на TUN + stats. Сохраняет outbounds/routing/balancers/dns как есть —
/// иначе теряется автовыбор и fallback (balancer MAIN → скрытый WHITE-хост).
pub fn build_raw(config: &RawXrayConfig, mtu: u32) -> String {
    let src = &config.root;
    let mut root = json!({
        "log": {"loglevel": "warning"},
        "stats": {},
        "api": {"tag": "api", "services": ["StatsService"]},
        "policy": {"system": {"statsOutboundUplink": true, "statsOutboundDownlink": true}},
        "inbounds": [tun_inbound(mtu), stats_inbound()],
    });
    let obj = root.as_object_mut().unwrap();
    // Сохраняем dns/routing/outbounds/balancers/observatory из подписки.
    for key in ["dns", "routing", "outbounds", "burstObservatory", "observatory"] {
        if let Some(v) = src.get(key) {
            obj.insert(key.to_string(), v.clone());
        }
    }
    // Правило: трафик api-инбаунда → api-сервис (иначе StatsService недоступен).
    inject_api_rule(obj);
    root.to_string()
}

/// Конфиг для прокси-пинга: локальный SOCKS-inbound + outbound под профиль, без
/// TUN/routing (Фаза 5). Ядро поднимается, клиент гонит HTTP через SOCKS.
#[allow(dead_code)] // используется в ping/ на Фазе 5
pub fn build_proxy_ping(config: &VlessConfig, socks_port: u16) -> String {
    json!({
        "log": {"loglevel": "none"},
        "inbounds": [{
            "tag": "socks", "protocol": "socks", "listen": "127.0.0.1", "port": socks_port,
            "settings": {"auth": "noauth", "udp": false}
        }],
        "outbounds": [vless_outbound(config)],
    })
    .to_string()
}

// ── строительные блоки ──

fn tun_inbound(mtu: u32) -> Value {
    json!({
        "tag": "tun",
        "protocol": "tun",
        "settings": {
            "name": TUN_NAME,
            "mtu": mtu,
            "gateway": [format!("{TUN_ADDRESS}/30")],
            "dns": ["1.1.1.1", "8.8.8.8"]
        },
        "sniffing": {"enabled": true, "destOverride": ["http", "tls", "quic"]}
    })
}

fn stats_inbound() -> Value {
    json!({
        "tag": "api-in",
        "protocol": "dokodemo-door",
        "listen": "127.0.0.1",
        "port": STATS_API_PORT,
        "settings": {"address": "127.0.0.1"}
    })
}

fn default_routing() -> Value {
    json!({
        "domainStrategy": "IPIfNonMatch",
        "rules": [
            // Трафик api-инбаунда → StatsService.
            {"type": "field", "inboundTag": ["api-in"], "outboundTag": "api"},
            // Приватные/локальные сети — напрямую (явные CIDR, без geoip.dat).
            {"type": "field", "outboundTag": "direct", "ip": [
                "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16",
                "127.0.0.0/8", "::1/128", "fc00::/7", "fe80::/10"
            ]}
            // Остальное — proxy по умолчанию (первый outbound).
        ]
    })
}

/// Добавляет правило api-инбаунда в существующий routing RawXray (или создаёт его).
fn inject_api_rule(obj: &mut serde_json::Map<String, Value>) {
    let api_rule = json!({"type": "field", "inboundTag": ["api-in"], "outboundTag": "api"});
    match obj.get_mut("routing").and_then(|r| r.as_object_mut()) {
        Some(routing) => {
            let rules = routing
                .entry("rules")
                .or_insert_with(|| Value::Array(vec![]));
            if let Some(arr) = rules.as_array_mut() {
                arr.insert(0, api_rule);
            }
        }
        None => {
            obj.insert(
                "routing".into(),
                json!({"domainStrategy": "IPIfNonMatch", "rules": [api_rule]}),
            );
        }
    }
}

fn vless_outbound(config: &VlessConfig) -> Value {
    let mut user = json!({"id": config.uuid, "encryption": "none"});
    if let Some(flow) = &config.flow {
        user["flow"] = json!(flow);
    }
    json!({
        "tag": "proxy",
        "protocol": "vless",
        "settings": {
            "vnext": [{"address": config.address, "port": config.port, "users": [user]}]
        },
        "streamSettings": stream_settings(config),
    })
}

fn stream_settings(config: &VlessConfig) -> Value {
    let network = match &config.transport {
        Transport::Tcp => "tcp",
        Transport::Ws { .. } => "ws",
        Transport::Grpc { .. } => "grpc",
        Transport::Xhttp { .. } => "xhttp",
    };
    let mut ss = json!({"network": network});
    let obj = ss.as_object_mut().unwrap();

    // --- security ---
    match &config.security {
        Security::None => {
            obj.insert("security".into(), json!("none"));
        }
        Security::Tls { sni, fingerprint, alpn, allow_insecure } => {
            obj.insert("security".into(), json!("tls"));
            let mut tls = json!({"allowInsecure": allow_insecure});
            let t = tls.as_object_mut().unwrap();
            if let Some(s) = sni { t.insert("serverName".into(), json!(s)); }
            if let Some(fp) = fingerprint { t.insert("fingerprint".into(), json!(fp)); }
            if let Some(a) = alpn { t.insert("alpn".into(), json!(a)); }
            obj.insert("tlsSettings".into(), tls);
        }
        Security::Reality { sni, fingerprint, public_key, short_id, spider_x } => {
            obj.insert("security".into(), json!("reality"));
            let mut r = json!({"publicKey": public_key});
            let ro = r.as_object_mut().unwrap();
            if let Some(s) = sni { ro.insert("serverName".into(), json!(s)); }
            if let Some(fp) = fingerprint { ro.insert("fingerprint".into(), json!(fp)); }
            if let Some(sid) = short_id { ro.insert("shortId".into(), json!(sid)); }
            if let Some(spx) = spider_x { ro.insert("spiderX".into(), json!(spx)); }
            obj.insert("realitySettings".into(), r);
        }
    }

    // --- transport-specific ---
    match &config.transport {
        Transport::Tcp => {}
        Transport::Ws { path, host } => {
            let mut ws = json!({});
            let w = ws.as_object_mut().unwrap();
            if let Some(p) = path { w.insert("path".into(), json!(p)); }
            if let Some(h) = host { w.insert("headers".into(), json!({"Host": h})); }
            obj.insert("wsSettings".into(), ws);
        }
        Transport::Grpc { service_name } => {
            let mut g = json!({});
            if let Some(sn) = service_name {
                g.as_object_mut().unwrap().insert("serviceName".into(), json!(sn));
            }
            obj.insert("grpcSettings".into(), g);
        }
        Transport::Xhttp { path, host, mode, extra } => {
            let mut xh = json!({});
            let x = xh.as_object_mut().unwrap();
            if let Some(p) = path { x.insert("path".into(), json!(p)); }
            if let Some(h) = host { x.insert("host".into(), json!(h)); }
            if let Some(m) = mode { x.insert("mode".into(), json!(m)); }
            // extra (xmux/xPadding/session/seq/…) — как есть.
            if let Some(e) = extra { x.insert("extra".into(), e.clone()); }
            obj.insert("xhttpSettings".into(), xh);
        }
    }

    ss
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_reality() -> VlessConfig {
        VlessConfig {
            remark: "Test".into(),
            address: "example.com".into(),
            port: 443,
            uuid: "00000000-0000-0000-0000-000000000000".into(),
            transport: Transport::Tcp,
            security: Security::Reality {
                sni: Some("example.com".into()),
                fingerprint: Some("chrome".into()),
                // Валидный x25519-публичный ключ (base64url) для -test ядра.
                public_key: "jNXHt1yRo0vDuchQlIP6Z0ZvjT3KtzVI-T4E7RoLJS0".into(),
                short_id: Some("0123abcd".into()),
                spider_x: None,
            },
            flow: Some("xtls-rprx-vision".into()),
        }
    }

    #[test]
    fn vless_reality_is_valid_json_with_expected_shape() {
        let json = build_vless(&sample_reality(), DEFAULT_MTU);
        let v: serde_json::Value = serde_json::from_str(&json).expect("валидный JSON");

        // tun-инбаунд присутствует.
        let inbounds = v["inbounds"].as_array().unwrap();
        assert!(inbounds.iter().any(|i| i["protocol"] == "tun"));
        // stats API-инбаунд.
        assert!(inbounds.iter().any(|i| i["protocol"] == "dokodemo-door"));
        // proxy-outbound vless + reality.
        let ob = &v["outbounds"][0];
        assert_eq!(ob["protocol"], "vless");
        assert_eq!(ob["streamSettings"]["security"], "reality");
        assert_eq!(ob["streamSettings"]["realitySettings"]["publicKey"],
                   "jNXHt1yRo0vDuchQlIP6Z0ZvjT3KtzVI-T4E7RoLJS0");
        // flow проброшен.
        assert_eq!(ob["settings"]["vnext"][0]["users"][0]["flow"], "xtls-rprx-vision");
    }

    #[test]
    fn xhttp_extra_passes_through_untouched() {
        let extra = serde_json::json!({"xmux": {"maxConcurrency": 8}, "xPadding": "100-200"});
        let cfg = VlessConfig {
            transport: Transport::Xhttp {
                path: Some("/sync".into()),
                host: Some("cdn.example.com".into()),
                mode: Some("auto".into()),
                extra: Some(extra.clone()),
            },
            security: Security::Tls {
                sni: Some("cdn.example.com".into()),
                fingerprint: Some("chrome".into()),
                alpn: None,
                allow_insecure: false,
            },
            ..sample_reality()
        };
        let json = build_vless(&cfg, DEFAULT_MTU);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let xh = &v["outbounds"][0]["streamSettings"]["xhttpSettings"];
        // extra проброшен байт-в-байт (инвариант XHTTP extra).
        assert_eq!(xh["extra"], extra);
        assert_eq!(xh["mode"], "auto");
    }
}
