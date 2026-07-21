//! Разбор Hysteria2-URI в `EngineConfig::Hysteria2`
//! (аналог Android `Hysteria2UriParser.kt`).
//!
//! Форматы: `hy2://<auth>@<host>:<port>?<params>#<remark>` / `hysteria2://...`
//! Параметры: sni, insecure(0/1), obfs(=salamander)+obfs-password, upmbps/downmbps.

use crate::engine::{EngineConfig, Hysteria2Config};

use super::uri;

pub fn parse(input: &str) -> Option<EngineConfig> {
    let trimmed = input.trim();
    let rest = trimmed
        .strip_prefix("hysteria2://")
        .or_else(|| trimmed.strip_prefix("hy2://"))?;

    let body = rest.split('#').next().unwrap_or(rest);
    let (before_query, query) = match body.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (body, None),
    };

    // auth@host:port; auth может отсутствовать (тогда берётся из params).
    let (auth, authority) = match before_query.split_once('@') {
        Some((a, rest)) => (uri::decode(a), rest),
        None => (String::new(), before_query),
    };

    let (host, port) = uri::parse_host_port(authority)?;
    if host.is_empty() {
        return None;
    }

    let params = uri::parse_query(query);
    let remark = uri::extract_remark(trimmed, &host);

    let obfs_type = params.get("obfs").map(|s| s.to_lowercase());
    let obfs_password = if obfs_type.as_deref() == Some("salamander") {
        params.get("obfs-password").or_else(|| params.get("obfsParam")).cloned()
    } else {
        None
    };

    let auth = if auth.is_empty() {
        params.get("auth").cloned().unwrap_or_default()
    } else {
        auth
    };

    Some(EngineConfig::Hysteria2(Hysteria2Config {
        remark,
        address: host,
        port,
        auth,
        sni: params.get("sni").or_else(|| params.get("peer")).cloned(),
        insecure: matches!(params.get("insecure").map(|s| s.as_str()), Some("1") | Some("true")),
        obfs_password,
        up_mbps: params.get("upmbps").or_else(|| params.get("up")).and_then(|s| s.parse().ok()),
        down_mbps: params.get("downmbps").or_else(|| params.get("down")).and_then(|s| s.parse().ok()),
    }))
}
