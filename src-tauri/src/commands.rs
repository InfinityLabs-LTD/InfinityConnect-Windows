//! Мост фронт↔бэк: `#[tauri::command]`-функции, вызываемые из фронта через invoke().
//!
//! Фаза 1: аккаунт и подписки — discovery, login/logout, список ключей и серверов.
//! Туннеля ещё нет (Фаза 2). Все команды возвращают `Result<_, AppError>` —
//! ошибка сериализуется во фронт как `{kind, message}`.

use serde::Serialize;
use tauri::State;

use crate::api::dto::{DiscoveryDto, KeyDto, UserInfoDto};
use crate::api::ApiClient;
use crate::error::AppResult;
use crate::subscription;

/// Тестовая команда Фазы 0: подтверждает мост invoke end-to-end.
#[tauri::command]
pub fn ping(name: String) -> String {
    format!("pong: привет, {name} (из Rust-бэкенда)")
}

/// Discovery по домену → базовый URL (сохраняется в клиенте и кэше).
#[tauri::command]
pub async fn discover(domain: String, api: State<'_, ApiClient>) -> AppResult<DiscoveryDto> {
    api.discover(&domain).await
}

/// Логин по логину/паролю. Токены сохраняются (DPAPI).
#[tauri::command]
pub async fn login(login: String, password: String, api: State<'_, ApiClient>) -> AppResult<()> {
    api.login(&login, &password).await
}

/// Разлогин: чистит токены.
#[tauri::command]
pub async fn logout(api: State<'_, ApiClient>) -> AppResult<()> {
    api.logout().await
}

/// Есть ли сохранённая авторизация (для стартового роутинга AUTH/HOME).
#[tauri::command]
pub async fn is_authorized(api: State<'_, ApiClient>) -> AppResult<bool> {
    Ok(api.is_authorized().await)
}

/// Данные аккаунта.
#[tauri::command]
pub async fn user_info(api: State<'_, ApiClient>) -> AppResult<UserInfoDto> {
    api.user_info().await
}

/// Список ключей (подписок) пользователя.
#[tauri::command]
pub async fn keys(api: State<'_, ApiClient>) -> AppResult<Vec<KeyDto>> {
    api.keys().await
}

/// Сервер подписки для UI: имя + адрес/порт + флаг протокола. Профиль (для
/// connect/пинга) на Фазе 1 не отдаём во фронт целиком — он останется в бэке.
#[derive(Debug, Serialize)]
pub struct SubscriptionServer {
    pub index: usize,
    pub remark: String,
    pub address: String,
    pub port: u16,
    pub protocol: String,
}

/// Список серверов ключа: грузит тело подписки, парсит в профили, отдаёт
/// сводку для UI (стиль Happ — список раскрыт). Первичен subscription_url;
/// при его отсутствии — серверы из `/v1/config/servers`.
#[tauri::command]
pub async fn key_servers(key_id: i64, api: State<'_, ApiClient>) -> AppResult<Vec<SubscriptionServer>> {
    let key = api.key(key_id).await?;

    // Первичный путь: подписка.
    if let Some(url) = key.subscription_url.as_deref().filter(|s| !s.is_empty()) {
        let body = api.subscription_body(url).await?;
        let configs = subscription::parse_subscription(&body.raw);
        if !configs.is_empty() {
            return Ok(configs
                .into_iter()
                .enumerate()
                .map(|(i, c)| SubscriptionServer {
                    index: i,
                    remark: c.remark().to_string(),
                    address: c.address().to_string(),
                    port: c.port(),
                    protocol: match c {
                        crate::engine::EngineConfig::Hysteria2(_) => "HYSTERIA2".into(),
                        _ => "VLESS".into(),
                    },
                })
                .collect());
        }
    }

    // Fallback: серверы из /v1/config/servers.
    let servers = api.servers(key_id).await?;
    Ok(servers
        .into_iter()
        .map(|s| SubscriptionServer {
            index: s.index as usize,
            remark: s.name.unwrap_or_else(|| s.server_address.clone().unwrap_or_default()),
            address: s.server_address.unwrap_or_default(),
            port: 0,
            protocol: "VLESS".into(),
        })
        .collect())
}
