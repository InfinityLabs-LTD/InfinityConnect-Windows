//! Разбор VLESS-URI в `EngineConfig::Vless` (аналог Android `VlessUriParser.kt`).
//!
//! Формат: `vless://<uuid>@<host>:<port>?<params>#<remark>`
//! Значимые параметры: type (транспорт), security, sni, fp, alpn, allowInsecure,
//! Reality: pbk/sid/spx, flow, транспорт: path/host/mode/serviceName/extra.

use std::collections::HashMap;

use crate::engine::{EngineConfig, Security, Transport, VlessConfig};

use super::uri;

pub fn parse(input: &str) -> Option<EngineConfig> {
    let trimmed = input.trim();
    let rest = trimmed.strip_prefix("vless://")?;

    // Отделяем remark (#...) и query (?...).
    let body = rest.split('#').next().unwrap_or(rest);
    let (before_query, query) = match body.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (body, None),
    };

    let (uuid, authority) = before_query.split_once('@')?;
    if uuid.is_empty() {
        return None;
    }
    let (host, port) = uri::parse_host_port(authority)?;
    if host.is_empty() {
        return None;
    }

    let params = uri::parse_query(query);
    let remark = uri::extract_remark(trimmed, &host);

    let transport = parse_transport(&params);
    let security = parse_security(&params);
    let flow = params.get("flow").filter(|s| !s.is_empty()).cloned();

    Some(EngineConfig::Vless(VlessConfig {
        remark,
        address: host,
        port,
        uuid: uuid.to_string(),
        transport,
        security,
        flow,
    }))
}

fn parse_transport(p: &HashMap<String, String>) -> Transport {
    match p.get("type").map(|s| s.to_lowercase()).as_deref() {
        Some("ws") | Some("websocket") => Transport::Ws {
            path: p.get("path").cloned(),
            host: p.get("host").cloned(),
        },
        Some("grpc") => Transport::Grpc {
            service_name: p.get("serviceName").or_else(|| p.get("servicename")).cloned(),
        },
        Some("xhttp") | Some("splithttp") => Transport::Xhttp {
            path: p.get("path").cloned(),
            host: p.get("host").cloned(),
            mode: p.get("mode").cloned(),
            // extra в URI — URL-decoded JSON (уже раскодирован parse_query).
            extra: parse_extra_json(p.get("extra")),
        },
        _ => Transport::Tcp,
    }
}

fn parse_security(p: &HashMap<String, String>) -> Security {
    match p.get("security").map(|s| s.to_lowercase()).as_deref() {
        Some("reality") => {
            let Some(pbk) = p.get("pbk").or_else(|| p.get("publicKey")).cloned() else {
                return Security::None;
            };
            Security::Reality {
                sni: p.get("sni").cloned(),
                fingerprint: Some(p.get("fp").cloned().unwrap_or_else(|| "chrome".into())),
                public_key: pbk,
                short_id: p.get("sid").cloned(),
                spider_x: p.get("spx").cloned(),
            }
        }
        Some("tls") => Security::Tls {
            sni: p.get("sni").cloned(),
            fingerprint: p.get("fp").cloned(),
            alpn: p.get("alpn").map(|a| {
                a.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
            }),
            allow_insecure: matches!(p.get("allowInsecure").map(|s| s.as_str()), Some("1") | Some("true")),
        },
        _ => Security::None,
    }
}

/// Разбирает значение `extra` (JSON-объект) из vless://-URI.
fn parse_extra_json(raw: Option<&String>) -> Option<serde_json::Value> {
    let text = raw?.trim();
    if !text.starts_with('{') {
        return None;
    }
    serde_json::from_str(text).ok()
}
