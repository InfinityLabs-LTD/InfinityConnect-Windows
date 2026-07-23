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
    /// База эндпоинта обновлений клиента (напр. `https://host/v1/client-updates`).
    /// Если задана — updater ходит сюда вместо статического endpoint из tauri.conf.json.
    #[serde(default)]
    pub update_base_url: Option<String>,
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

/// Запрос обмена одноразового кода (auth через сайт, deep-link) на токены.
#[derive(Debug, Serialize)]
pub struct ExchangeCodeRequestDto {
    pub code: String,
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

/// Текущая дата UTC как `YYYY-MM-DD` (civil-from-days, алгоритм Хиннанта —
/// точная григорианская конверсия без внешних зависимостей).
fn today_utc_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs() as i64)
        .unwrap_or(0);
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

impl KeyDto {
    /// Достигнут лимит устройств (данные есть и исчерпаны). Как в Android:
    /// device_limit с фолбэком на hwid_device_limit.
    fn devices_exhausted(&self) -> bool {
        let limit = self.device_limit.or(self.hwid_device_limit).unwrap_or(0);
        let used = self.devices_used.or(self.hwid_devices_used).unwrap_or(0);
        limit > 0 && used >= limit
    }

    /// Истёк ли срок ключа. ISO-8601 (`YYYY-MM-DD…`) сравнима лексикографически,
    /// поэтому достаточно сравнить префикс даты со «вчера» (щадяще к TZ):
    /// непарсибельное значение считаем не истёкшим.
    fn expired(&self) -> bool {
        let Some(raw) = self.expires_at.as_deref() else { return false };
        if raw.len() < 10 {
            return false;
        }
        let key_date = &raw[..10]; // YYYY-MM-DD
        if !key_date.as_bytes()[..4].iter().all(u8::is_ascii_digit) {
            return false;
        }
        key_date < today_utc_date().as_str()
    }

    /// Причина блокировки подключения (зеркало Android VpnKey.status()):
    /// приоритет — статус сервера; иначе выводим по датам/активности/лимиту.
    /// `None` — ключ доступен (ACTIVE).
    pub fn blocked_reason(&self) -> Option<&'static str> {
        match self.status.as_deref().map(|s| s.trim().to_uppercase()) {
            Some(s) if s == "ACTIVE" => return None,
            Some(s) if s == "EXPIRED" => return Some("Срок подписки истёк"),
            Some(s) if s == "DISABLED" => return Some("Подписка отключена"),
            Some(s) if s == "LIMITED" => {
                return Some(if self.devices_exhausted() {
                    "Достигнут лимит устройств этой подписки"
                } else {
                    "Достигнут лимит этой подписки"
                })
            }
            _ => {}
        }
        if self.expired() {
            Some("Срок подписки истёк")
        } else if !self.is_active {
            Some("Подписка отключена")
        } else if self.devices_exhausted() {
            Some("Достигнут лимит устройств этой подписки")
        } else {
            None
        }
    }
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

#[cfg(test)]
mod key_status_tests {
    use super::*;

    fn key() -> KeyDto {
        serde_json::from_str(r#"{"id": 1, "is_active": true}"#).unwrap()
    }

    #[test]
    fn active_key_not_blocked() {
        let mut k = key();
        k.expires_at = Some("2099-01-01T00:00:00Z".into());
        assert_eq!(k.blocked_reason(), None);
    }

    #[test]
    fn server_status_has_priority() {
        let mut k = key();
        k.status = Some("DISABLED".into());
        assert_eq!(k.blocked_reason(), Some("Подписка отключена"));
        // ACTIVE от сервера перекрывает клиентские признаки.
        k.status = Some("ACTIVE".into());
        k.is_active = false;
        assert_eq!(k.blocked_reason(), None);
    }

    #[test]
    fn expired_and_inactive_and_device_limit() {
        let mut k = key();
        k.expires_at = Some("2020-01-01".into());
        assert_eq!(k.blocked_reason(), Some("Срок подписки истёк"));

        let mut k = key();
        k.is_active = false;
        assert_eq!(k.blocked_reason(), Some("Подписка отключена"));

        let mut k = key();
        k.device_limit = Some(2);
        k.devices_used = Some(2);
        assert_eq!(k.blocked_reason(), Some("Достигнут лимит устройств этой подписки"));

        // hwid-фолбэк как в Android.
        let mut k = key();
        k.hwid_device_limit = Some(1);
        k.hwid_devices_used = Some(3);
        assert_eq!(k.blocked_reason(), Some("Достигнут лимит устройств этой подписки"));
    }

    #[test]
    fn today_date_is_iso() {
        let d = today_utc_date();
        assert_eq!(d.len(), 10);
        assert_eq!(&d[4..5], "-");
        assert!(d.as_str() > "2025-01-01" && d.as_str() < "2100-01-01");
    }
}
