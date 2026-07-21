//! Построение `EngineConfig` для подключения (аналог Android `BuildConnectionUseCase`).
//!
//! Стратегия: subscription_url первичен (единственный надёжный путь для XHTTP и
//! Hysteria2); `/v1/config` — fallback для VLESS-метаданных, когда подписки нет.

use crate::api::dto::ConfigDto;
use crate::api::ApiClient;
use crate::engine::{EngineConfig, Security, Transport, VlessConfig};
use crate::error::{AppError, AppResult};
use crate::subscription;

/// Готовит профиль сервера ключа по индексу.
pub async fn build_connection(
    api: &ApiClient,
    key_id: i64,
    server_index: usize,
) -> AppResult<EngineConfig> {
    let key = api.key(key_id).await?;

    // 1. Первичный путь: подписка.
    if let Some(url) = key.subscription_url.as_deref().filter(|s| !s.is_empty()) {
        if let Ok(body) = api.subscription_body(url).await {
            let configs = subscription::parse_subscription(&body.raw);
            if !configs.is_empty() {
                let idx = if server_index < configs.len() { server_index } else { 0 };
                return Ok(configs.into_iter().nth(idx).unwrap());
            }
        }
    }

    // 2. Fallback: /v1/config (только VLESS).
    let dto = api.config(key_id, server_index as i32).await?;

    // Приоритетно — raw_uri (в нём вся правда о транспорте).
    if let Some(uri) = dto.raw_uri.as_deref().filter(|s| !s.is_empty()) {
        if let Some(cfg) = subscription::parse_single_uri(uri) {
            return Ok(cfg);
        }
    }

    // Иначе — собираем VLESS из полей DTO.
    build_vless_from_dto(&dto, key.name.as_deref())
}

fn build_vless_from_dto(dto: &ConfigDto, key_name: Option<&str>) -> AppResult<EngineConfig> {
    let address = dto
        .server_address
        .clone()
        .ok_or_else(|| AppError::Parse("нет адреса сервера в конфиге".into()))?;
    let port = dto
        .server_port
        .ok_or_else(|| AppError::Parse("нет порта сервера в конфиге".into()))?;
    let uuid = dto
        .uuid
        .clone()
        .ok_or_else(|| AppError::Parse("нет UUID в конфиге".into()))?;

    let transport = match dto.network.as_deref().map(str::to_lowercase).as_deref() {
        Some("ws") | Some("websocket") => Transport::Ws {
            path: dto.path.clone(),
            host: dto.host.clone(),
        },
        Some("grpc") => Transport::Grpc { service_name: dto.path.clone() },
        Some("xhttp") | Some("splithttp") => Transport::Xhttp {
            path: dto.path.clone(),
            host: dto.host.clone(),
            mode: None,
            extra: None,
        },
        _ => Transport::Tcp,
    };

    let security = match dto.security.as_deref().map(str::to_lowercase).as_deref() {
        Some("reality") => {
            let public_key = dto
                .public_key
                .clone()
                .ok_or_else(|| AppError::Parse("Reality без public_key".into()))?;
            Security::Reality {
                sni: dto.sni.clone(),
                fingerprint: Some(dto.fingerprint.clone().unwrap_or_else(|| "chrome".into())),
                public_key,
                short_id: dto.short_id.clone(),
                spider_x: None,
            }
        }
        Some("tls") => Security::Tls {
            sni: dto.sni.clone(),
            fingerprint: dto.fingerprint.clone(),
            alpn: None,
            allow_insecure: false,
        },
        _ => Security::None,
    };

    Ok(EngineConfig::Vless(VlessConfig {
        remark: key_name.unwrap_or(&address).to_string(),
        address,
        port,
        uuid,
        transport,
        security,
        flow: dto.flow.clone().filter(|s| !s.is_empty()),
    }))
}
