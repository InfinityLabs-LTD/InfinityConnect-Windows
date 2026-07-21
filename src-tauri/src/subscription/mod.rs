//! Парсер подписки Remnawave (аналог Android `SubscriptionParser.kt`).
//!
//! Принимает сырое тело подписки и возвращает список `EngineConfig` — по одному
//! профилю на сервер. Форматы тела (в порядке проверки):
//!  1. JSON-массив готовых Xray-конфигов (панель отдаёт клиенту Happ с HWID).
//!  2. base64 со списком URI (vless://, hy2://), по одному на строку.
//!  3. Список URI в открытом виде.

mod hysteria2_uri;
mod uri;
mod vless_uri;

use base64::Engine as _;
use serde_json::Value;

use crate::engine::{
    EngineConfig, Hysteria2Config, Security, Transport, VlessConfig,
};

/// Proxy-протоколы, считаемые «сервером» в JSON-конфиге панели.
const PROXY_PROTOCOLS: [&str; 3] = ["vless", "hysteria", "hysteria2"];

/// Разбирает полное тело подписки в список профилей.
pub fn parse_subscription(raw: &str) -> Vec<EngineConfig> {
    let trimmed = raw.trim();

    // 1. JSON-массив/объект конфигов Xray.
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        let configs = parse_json_configs(trimmed);
        if !configs.is_empty() {
            return configs;
        }
    }

    // 2/3. base64 или открытый список URI.
    let decoded = maybe_base64_decode(trimmed);
    let d = decoded.trim();
    if d.starts_with('[') || d.starts_with('{') {
        let configs = parse_json_configs(d);
        if !configs.is_empty() {
            return configs;
        }
    }

    decoded
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter_map(parse_single_uri)
        .collect()
}

/// Разбирает единичный URI (raw_uri из /v1/config или ключа).
pub fn parse_single_uri(u: &str) -> Option<EngineConfig> {
    if u.starts_with("vless://") {
        vless_uri::parse(u)
    } else if u.starts_with("hy2://") || u.starts_with("hysteria2://") {
        hysteria2_uri::parse(u)
    } else {
        None
    }
}

// ── JSON-конфиги Xray ──

fn parse_json_configs(text: &str) -> Vec<EngineConfig> {
    let Ok(root) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let array = match root {
        Value::Array(a) => a,
        obj @ Value::Object(_) => vec![obj],
        _ => return Vec::new(),
    };
    array.iter().filter_map(parse_single_json_config).collect()
}

fn parse_single_json_config(config: &Value) -> Option<EngineConfig> {
    let outbounds = config.get("outbounds")?.as_array()?;
    // Все proxy-outbounds (vless/hysteria). Пропускаем direct/block/freedom.
    let proxies: Vec<&Value> = outbounds
        .iter()
        .filter(|ob| {
            ob.get("protocol")
                .and_then(Value::as_str)
                .map(|p| PROXY_PROTOCOLS.contains(&p))
                .unwrap_or(false)
        })
        .collect();
    let proxy = *proxies.first()?;

    let remark = str_field(config, "remarks").or_else(|| str_field(config, "remark"));

    // «Сложный» конфиг (balancer/несколько outbound) → RawXray целиком.
    if is_complex_config(config, &proxies) {
        let primary = match parse_vless_outbound(proxy, remark.clone()) {
            Some(EngineConfig::Vless(v)) => Some(v),
            _ => None,
        };
        let fallback = primary.as_ref().map(|p| p.address.clone());
        return Some(EngineConfig::RawXray(crate::engine::RawXrayConfig {
            remark: remark.or(fallback).unwrap_or_else(|| "Авто".into()),
            root: config.clone(),
            primary_outbound: primary,
        }));
    }

    match proxy.get("protocol").and_then(Value::as_str) {
        Some("hysteria") | Some("hysteria2") => parse_hysteria_outbound(proxy, remark),
        _ => parse_vless_outbound(proxy, remark),
    }
}

/// Конфиг «сложный», если его нельзя свести к одному outbound: несколько
/// proxy-outbounds, либо в routing есть balancers/rule с balancerTag.
fn is_complex_config(config: &Value, proxies: &[&Value]) -> bool {
    if proxies.len() > 1 {
        return true;
    }
    let Some(routing) = config.get("routing") else {
        return false;
    };
    if routing
        .get("balancers")
        .and_then(Value::as_array)
        .map(|b| !b.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    routing
        .get("rules")
        .and_then(Value::as_array)
        .map(|rules| rules.iter().any(|r| r.get("balancerTag").is_some()))
        .unwrap_or(false)
}

fn parse_vless_outbound(proxy: &Value, remark: Option<String>) -> Option<EngineConfig> {
    let settings = proxy.get("settings")?;
    let vnext = settings.get("vnext")?.as_array()?.first()?;
    let address = str_field(vnext, "address")?;
    let port = u16_field(vnext, "port")?;
    let user = vnext.get("users")?.as_array()?.first()?;
    let uuid = str_field(user, "id")?;
    let flow = str_field(user, "flow").filter(|s| !s.is_empty());

    let stream = proxy.get("streamSettings");
    Some(EngineConfig::Vless(VlessConfig {
        remark: remark.unwrap_or_else(|| address.clone()),
        address,
        port,
        uuid,
        transport: parse_transport(stream),
        security: parse_security(stream),
        flow,
    }))
}

/// Разбирает hysteria(2)-outbound из JSON-конфига панели.
fn parse_hysteria_outbound(proxy: &Value, remark: Option<String>) -> Option<EngineConfig> {
    let settings = proxy.get("settings")?;
    let address = str_field(settings, "address")?;
    let port = u16_field(settings, "port")?;

    let stream = proxy.get("streamSettings");
    let hy = stream.and_then(|s| s.get("hysteriaSettings"));
    let tls = stream.and_then(|s| s.get("tlsSettings"));

    let auth = hy
        .and_then(|h| str_field(h, "auth"))
        .or_else(|| str_field(settings, "auth"))
        .unwrap_or_default();

    let obfs_type = hy.and_then(|h| str_field(h, "obfs")).map(|s| s.to_lowercase());
    let obfs_password = if obfs_type.as_deref() == Some("salamander")
        || hy.map(|h| h.get("obfsPassword").is_some()).unwrap_or(false)
    {
        hy.and_then(|h| str_field(h, "obfsPassword").or_else(|| str_field(h, "obfs-password")))
    } else {
        None
    };

    Some(EngineConfig::Hysteria2(Hysteria2Config {
        remark: remark.unwrap_or_else(|| address.clone()),
        address,
        port,
        auth,
        sni: tls.and_then(|t| str_field(t, "serverName")).or_else(|| hy.and_then(|h| str_field(h, "sni"))),
        insecure: bool_str(tls.and_then(|t| t.get("allowInsecure")))
            || bool_str(hy.and_then(|h| h.get("insecure"))),
        obfs_password,
        up_mbps: None,
        down_mbps: None,
    }))
}

fn parse_transport(stream: Option<&Value>) -> Transport {
    let network = stream
        .and_then(|s| str_field(s, "network"))
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "tcp".into());
    match network.as_str() {
        "ws" | "websocket" => {
            let ws = stream.and_then(|s| s.get("wsSettings"));
            Transport::Ws {
                path: ws.and_then(|w| str_field(w, "path")),
                host: ws
                    .and_then(|w| w.get("headers"))
                    .and_then(|h| str_field(h, "Host")),
            }
        }
        "grpc" => Transport::Grpc {
            service_name: stream
                .and_then(|s| s.get("grpcSettings"))
                .and_then(|g| str_field(g, "serviceName")),
        },
        "xhttp" | "splithttp" => {
            let xh = stream
                .and_then(|s| s.get("xhttpSettings").or_else(|| s.get("splithttpSettings")));
            Transport::Xhttp {
                path: xh.and_then(|x| str_field(x, "path")),
                host: xh.and_then(|x| str_field(x, "host")),
                mode: xh.and_then(|x| str_field(x, "mode")),
                // extra несём как есть — сервер белых списков сверяет эти поля.
                extra: xh.and_then(|x| x.get("extra")).cloned(),
            }
        }
        _ => Transport::Tcp,
    }
}

fn parse_security(stream: Option<&Value>) -> Security {
    match stream
        .and_then(|s| str_field(s, "security"))
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("reality") => {
            let r = stream.and_then(|s| s.get("realitySettings"));
            let Some(pbk) = r.and_then(|r| str_field(r, "publicKey")) else {
                return Security::None;
            };
            Security::Reality {
                sni: r.and_then(|r| str_field(r, "serverName")),
                fingerprint: Some(
                    r.and_then(|r| str_field(r, "fingerprint")).unwrap_or_else(|| "chrome".into()),
                ),
                public_key: pbk,
                short_id: r.and_then(|r| str_field(r, "shortId")),
                spider_x: r.and_then(|r| str_field(r, "spiderX")),
            }
        }
        Some("tls") => {
            let t = stream.and_then(|s| s.get("tlsSettings"));
            Security::Tls {
                sni: t.and_then(|t| str_field(t, "serverName")),
                fingerprint: t.and_then(|t| str_field(t, "fingerprint")),
                alpn: None,
                allow_insecure: bool_str(t.and_then(|t| t.get("allowInsecure"))),
            }
        }
        _ => Security::None,
    }
}

// ── base64 ──

fn maybe_base64_decode(raw: &str) -> String {
    if raw.contains("://") {
        return raw.to_string();
    }
    let candidate: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    // Пробуем стандартный и URL-safe алфавиты (no-pad допускаем).
    for engine in [
        &base64::engine::general_purpose::STANDARD_NO_PAD,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
    ] {
        if let Ok(bytes) = engine.decode(candidate.trim_end_matches('=')) {
            if let Ok(text) = String::from_utf8(bytes) {
                if text.contains("://") || text.trim_start().starts_with('[') {
                    return text;
                }
            }
        }
    }
    raw.to_string()
}

// ── хелперы извлечения примитивов ──

/// Строковое поле: принимает и JSON-строку, и число (как в Kotlin `contentOrNull`).
fn str_field(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Порт: из числа или из строки.
fn u16_field(v: &Value, key: &str) -> Option<u16> {
    match v.get(key)? {
        Value::Number(n) => n.as_u64().and_then(|x| u16::try_from(x).ok()),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// `allowInsecure`/`insecure`: сервер иногда шлёт строку "true", иногда bool.
fn bool_str(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}
