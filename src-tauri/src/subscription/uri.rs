//! Общие хелперы разбора URI (аналог Android `UriParsing.kt`).

use std::collections::HashMap;

/// Процентное декодирование (`%xx`) + замена `+` на пробел в query-значениях.
pub fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_val(bytes[i + 1]);
                let lo = hex_val(bytes[i + 2]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push(h << 4 | l);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Разбирает query-строку `k=v&k2=v2` в map с раскодированными значениями.
pub fn parse_query(query: Option<&str>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(q) = query else { return map };
    for pair in q.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        map.insert(decode(k), decode(v));
    }
    map
}

/// Извлекает remark (`#...`) из URI, декодирует; при отсутствии — fallback.
pub fn extract_remark(uri: &str, fallback: &str) -> String {
    match uri.split_once('#') {
        Some((_, frag)) if !frag.is_empty() => decode(frag),
        _ => fallback.to_string(),
    }
}

/// Разбирает `host:port`, поддерживая IPv6 в скобках `[::1]:443`.
pub fn parse_host_port(authority: &str) -> Option<(String, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let close = rest.find(']')?;
        let host = &rest[..close];
        let after = &rest[close + 1..];
        let port = after.strip_prefix(':')?.parse().ok()?;
        return Some((host.to_string(), port));
    }
    let colon = authority.rfind(':')?;
    if colon == 0 {
        return None;
    }
    let host = &authority[..colon];
    let port = authority[colon + 1..].parse().ok()?;
    Some((host.to_string(), port))
}
