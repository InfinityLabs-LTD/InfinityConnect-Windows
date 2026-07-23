//! DTO ответов сервера (аналог Android `data/remote/dto/*.kt`).
//! Все опциональные поля — с `#[serde(default)]`, т.к. API их отдаёт не всегда.

use serde::{Deserialize, Deserializer, Serialize};

/// Панель отдаёт числовые id как СТРОКИ (`"3"`, `"577633336"`). Принимаем и
/// строку, и число → i64 (иначе serde падает `error decoding response body`).
fn de_flex_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrInt {
        Int(i64),
        Str(String),
    }
    match StrOrInt::deserialize(d)? {
        StrOrInt::Int(n) => Ok(n),
        StrOrInt::Str(s) => s.trim().parse().map_err(serde::de::Error::custom),
    }
}

/// То же, но для опционального поля (`null`/отсутствует → None).
fn de_flex_opt_i64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrIntOrNull {
        Int(i64),
        Str(String),
    }
    match Option::<StrOrIntOrNull>::deserialize(d)? {
        None => Ok(None),
        Some(StrOrIntOrNull::Int(n)) => Ok(Some(n)),
        Some(StrOrIntOrNull::Str(s)) if s.trim().is_empty() => Ok(None),
        Some(StrOrIntOrNull::Str(s)) => s.trim().parse().map(Some).map_err(serde::de::Error::custom),
    }
}

// ── Discovery ──

/// Ответ публичного `GET /v1/discovery`. Клиент знает только домен, остальное — отсюда.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryDto {
    pub api_base_url: String,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub register_url: Option<String>,
    #[serde(default)]
    pub forgot_password_url: Option<String>,
    #[serde(default)]
    pub support_url: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub api_version: Option<i32>,
    #[serde(default)]
    pub features: Option<DiscoveryFeaturesDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFeaturesDto {
    #[serde(default)]
    pub trial_enabled: bool,
    #[serde(default)]
    pub referrals_enabled: bool,
}

// ── Auth ──

#[derive(Debug, Serialize)]
pub struct LoginRequestDto {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshRequestDto {
    pub refresh_token: String,
}

/// Ответ login/refresh: пара HMAC-подписанных токенов (access ~1ч, refresh ~30д).
/// Поля срока — для проактивного refresh на след. фазах (пока не читаются).
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponseDto {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub refresh_expires_in_days: Option<i32>,
}

// ── Keys ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysResponseDto {
    #[serde(default)]
    pub keys: Vec<KeyDto>,
}

/// Один ключ (подписка Remnawave). `subscription_url` — первичный источник
/// конфигов для XHTTP/Hysteria2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDto {
    #[serde(deserialize_with = "de_flex_i64")]
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub server_address: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub country_flag: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub used_traffic_bytes: Option<i64>,
    #[serde(default)]
    pub traffic_limit_bytes: Option<i64>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub subscription_url: Option<String>,
    #[serde(default)]
    pub is_premium: bool,
    /// ACTIVE / EXPIRED / DISABLED / LIMITED.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub device_limit: Option<i32>,
    #[serde(default)]
    pub hwid_device_limit: Option<i32>,
    #[serde(default)]
    pub devices_used: Option<i32>,
    #[serde(default)]
    pub hwid_devices_used: Option<i32>,
}

// ── Config/servers ──

#[derive(Debug, Clone, Deserialize)]
pub struct ServersResponseDto {
    #[serde(default)]
    pub servers: Vec<ServerEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntryDto {
    pub index: i32,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub flag: Option<String>,
    #[serde(default)]
    pub server_address: Option<String>,
}

/// Ответ `GET /v1/config`. Сервер надёжно разбирает только VLESS; для XHTTP/Hy2
/// первичен raw_uri/subscription_url. Все поля кроме raw_uri опциональны.
/// Используется в fallback-пути BuildConnectionUseCase (Фаза 2).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigDto {
    #[serde(default)]
    pub server_address: Option<String>,
    #[serde(default)]
    pub server_port: Option<u16>,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
    #[serde(default)]
    pub security: Option<String>,
    #[serde(default)]
    pub sni: Option<String>,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub short_id: Option<String>,
    #[serde(default)]
    pub flow: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub raw_uri: Option<String>,
}

// ── User ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoDto {
    #[serde(default, deserialize_with = "de_flex_opt_i64")]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub is_subscription_active: bool,
    #[serde(default)]
    pub subscription_expires_at: Option<String>,
    #[serde(default)]
    pub plan_name: Option<String>,
}

/// Ответ `GET /v1/user/subscription` — агрегированные данные по подписке
/// (зеркало Android SubscriptionInfoDto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfoDto {
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub keys_count: i32,
    #[serde(default)]
    pub latest_expiry: Option<String>,
    /// Наименьшая дата окончания среди активных ключей — реальный «ближайший» срок.
    #[serde(default)]
    pub earliest_expiry: Option<String>,
    #[serde(default)]
    pub total_spent: Option<f64>,
    #[serde(default)]
    pub total_months: Option<i32>,
}
