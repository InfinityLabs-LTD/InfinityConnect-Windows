//! HTTP-клиент к серверу InfinityConnect (аналог Android `data/remote`).
//!
//! Один `ApiClient` держит discovery-`base_url` и токены. Авторизация: Bearer на
//! защищённых запросах; при 401 — один refresh и повтор (аналог
//! `AuthInterceptor` + `TokenAuthenticator`). Тело подписки грузится с
//! HWID-заголовками клиента Happ (иначе панель отдаёт заглушку).

pub mod dto;

use std::sync::Arc;

use reqwest::{Client, StatusCode};
use tokio::sync::RwLock;

use crate::device;
use crate::error::{AppError, AppResult};
use crate::store::{self, Tokens};

use dto::*;

/// Клиент API. Клонируется дёшево (Arc внутри); общее состояние под RwLock.
#[derive(Clone)]
pub struct ApiClient {
    http: Client,
    state: Arc<RwLock<ApiState>>,
}

#[derive(Default)]
struct ApiState {
    /// Базовый URL из discovery (например `https://host/v1`). Без хвостового `/`.
    base_url: Option<String>,
    tokens: Option<Tokens>,
}

/// Тело подписки + интервал обновления (часы) из заголовка Remnawave.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionBody {
    pub raw: String,
    pub update_interval_hours: Option<u32>,
}

impl ApiClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .user_agent(device::USER_AGENT)
            .build()
            .expect("reqwest client");
        let client = Self {
            http,
            state: Arc::new(RwLock::new(ApiState::default())),
        };
        // Восстанавливаем токены и discovery из офлайн-кэша при старте.
        if let Ok(Some(tokens)) = store::load_tokens() {
            if let Ok(mut s) = client.state.try_write() {
                s.tokens = Some(tokens);
            }
        }
        if let Some(disc) = store::read_cache::<DiscoveryDto>(store::CACHE_DISCOVERY) {
            if let Ok(mut s) = client.state.try_write() {
                s.base_url = Some(normalize_base(&disc.api_base_url));
            }
        }
        client
    }

    /// Есть ли сохранённые токены (для стартового роутинга AUTH/HOME).
    pub async fn is_authorized(&self) -> bool {
        self.state.read().await.tokens.is_some()
    }

    /// Фоновое обновление подписок: тянет ключи и перекачивает тела подписок
    /// (обновляет зашифрованный кэш). Тихо игнорирует ошибки (офлайн/нет сессии).
    /// Возвращает число обновлённых подписок.
    pub async fn refresh_subscriptions(&self) -> usize {
        if !self.is_authorized().await {
            return 0;
        }
        let Ok(keys) = self.keys().await else { return 0 };
        let mut n = 0;
        for key in keys {
            if let Some(url) = key.subscription_url.as_deref().filter(|s| !s.is_empty()) {
                if self.subscription_body(url).await.is_ok() {
                    n += 1;
                }
            }
        }
        n
    }

    // ── Discovery ──

    /// Discovery по домену: `https://<domain>/v1/discovery` → base_url.
    pub async fn discover(&self, domain: &str) -> AppResult<DiscoveryDto> {
        // Принимаем и голый "host:port", и полный "https://host:port/" — нормализуем.
        let host = domain
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_matches('/');
        let url = format!("https://{host}/v1/discovery");
        let dto: DiscoveryDto = self.http.get(&url).send().await?.error_for_status()?.json().await?;
        {
            let mut s = self.state.write().await;
            s.base_url = Some(normalize_base(&dto.api_base_url));
        }
        let _ = store::write_cache(store::CACHE_DISCOVERY, &dto);
        Ok(dto)
    }

    async fn base(&self) -> AppResult<String> {
        self.state
            .read()
            .await
            .base_url
            .clone()
            .ok_or_else(|| AppError::Other("api_base_url не задан: выполните discovery".into()))
    }

    /// Регистрируемый домен из base_url discovery (напр. `infinityconnect.ru`) —
    /// корень доверия для проверки URL подписки. `None`, если discovery не было.
    async fn trusted_domain(&self) -> Option<String> {
        let base = self.state.read().await.base_url.clone()?;
        let host = base
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()?
            .split(':')
            .next()?;
        registrable_domain(host)
    }

    // ── Auth ──

    /// Логин: сохраняет токены (DPAPI) при успехе.
    pub async fn login(&self, login: &str, password: &str) -> AppResult<()> {
        let base = self.base().await?;
        let body = LoginRequestDto {
            login: login.to_string(),
            password: password.to_string(),
        };
        let dto: TokenResponseDto = self
            .http
            .post(format!("{base}/auth/login"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        self.set_tokens(dto.access_token, dto.refresh_token).await
    }

    /// Обмен одноразового кода (auth через сайт → deep-link) на пару токенов.
    /// Код короткоживущий и одноразовый — выдаёт его сайт после входа.
    pub async fn exchange_auth_code(&self, code: &str) -> AppResult<()> {
        let base = self.base().await?;
        let dto: TokenResponseDto = self
            .http
            .post(format!("{base}/auth/desktop/exchange"))
            .json(&ExchangeCodeRequestDto { code: code.to_string() })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        self.set_tokens(dto.access_token, dto.refresh_token).await
    }

    /// URL страницы входа на сайте для desktop-авторизации (deep-link возврат).
    /// База — site_url из discovery-кэша; None, если discovery ещё не было.
    pub fn site_auth_url(&self) -> Option<String> {
        let site = store::read_cache::<DiscoveryDto>(store::CACHE_DISCOVERY)?.site_url?;
        Some(format!("{}/auth/desktop", site.trim_end_matches('/')))
    }

    /// Разлогин: чистит токены локально (+ best-effort вызов сервера).
    pub async fn logout(&self) -> AppResult<()> {
        if let Ok(base) = self.base().await {
            let token = self.state.read().await.tokens.as_ref().map(|t| t.access.clone());
            if let Some(access) = token {
                let _ = self
                    .http
                    .post(format!("{base}/auth/logout"))
                    .bearer_auth(access)
                    .send()
                    .await;
            }
        }
        self.clear_tokens().await
    }

    async fn set_tokens(&self, access: String, refresh: String) -> AppResult<()> {
        let tokens = Tokens { access, refresh };
        store::save_tokens(&tokens)?;
        self.state.write().await.tokens = Some(tokens);
        Ok(())
    }

    async fn clear_tokens(&self) -> AppResult<()> {
        store::clear_tokens()?;
        self.state.write().await.tokens = None;
        Ok(())
    }

    /// Пытается обновить access через refresh_token. `Err(Unauthorized)` → разлогин.
    async fn refresh(&self) -> AppResult<()> {
        let base = self.base().await?;
        let refresh_token = self
            .state
            .read()
            .await
            .tokens
            .as_ref()
            .map(|t| t.refresh.clone())
            .ok_or(AppError::Unauthorized)?;

        let resp = self
            .http
            .post(format!("{base}/auth/refresh"))
            .json(&RefreshRequestDto { refresh_token })
            .send()
            .await?;
        if !resp.status().is_success() {
            self.clear_tokens().await?;
            return Err(AppError::Unauthorized);
        }
        let dto: TokenResponseDto = resp.json().await?;
        self.set_tokens(dto.access_token, dto.refresh_token).await
    }

    /// GET защищённого JSON-эндпоинта с авто-refresh при 401 (одна попытка).
    async fn get_auth<T: serde::de::DeserializeOwned>(&self, path: &str) -> AppResult<T> {
        let base = self.base().await?;
        let url = format!("{base}/{path}");
        for attempt in 0..2 {
            let access = self
                .state
                .read()
                .await
                .tokens
                .as_ref()
                .map(|t| t.access.clone())
                .ok_or(AppError::Unauthorized)?;
            let resp = self.http.get(&url).bearer_auth(access).send().await?;
            if resp.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                self.refresh().await?;
                continue;
            }
            return Ok(resp.error_for_status()?.json().await?);
        }
        Err(AppError::Unauthorized)
    }

    // ── Аккаунт / ключи / серверы / конфиг ──

    pub async fn user_info(&self) -> AppResult<UserInfoDto> {
        self.get_auth("user/info").await
    }

    /// Агрегированные данные подписки (срок/ключи/месяцы/потрачено) для профиля.
    pub async fn subscription_info(&self) -> AppResult<SubscriptionInfoDto> {
        self.get_auth("user/subscription").await
    }

    /// support_url из офлайн-кэша discovery (для кнопки «Написать в поддержку»).
    pub fn support_url(&self) -> Option<String> {
        store::read_cache::<DiscoveryDto>(store::CACHE_DISCOVERY)?.support_url
    }

    /// Список ключей. Кэшируется для офлайн-режима.
    pub async fn keys(&self) -> AppResult<Vec<KeyDto>> {
        match self.get_auth::<KeysResponseDto>("keys").await {
            Ok(resp) => {
                let _ = store::write_cache_secure(store::CACHE_KEYS, &resp.keys);
                Ok(resp.keys)
            }
            Err(e @ AppError::Network(_)) => {
                // Офлайн: отдаём кэш, если есть.
                if let Some(cached) = store::read_cache_secure::<Vec<KeyDto>>(store::CACHE_KEYS) {
                    Ok(cached)
                } else {
                    Err(e)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Один ключ по id (из кэша, если офлайн).
    pub async fn key(&self, id: i64) -> AppResult<KeyDto> {
        match self.get_auth::<KeyDto>(&format!("keys/{id}")).await {
            Ok(k) => Ok(k),
            Err(e @ AppError::Network(_)) => store::read_cache_secure::<Vec<KeyDto>>(store::CACHE_KEYS)
                .and_then(|ks| ks.into_iter().find(|k| k.id == id))
                .ok_or(e),
            Err(e) => Err(e),
        }
    }

    pub async fn servers(&self, key_id: i64) -> AppResult<Vec<ServerEntryDto>> {
        let resp: ServersResponseDto = self.get_auth(&format!("config/servers?key_id={key_id}")).await?;
        Ok(resp.servers)
    }

    // Фаза 2: fallback /v1/config для BuildConnectionUseCase (пока не вызывается).
    #[allow(dead_code)]
    pub async fn config(&self, key_id: i64, server_index: i32) -> AppResult<ConfigDto> {
        self.get_auth(&format!("config?key_id={key_id}&server={server_index}")).await
    }

    // ── Тело подписки (HWID-заголовки обязательны) ──

    /// Грузит тело подписки с заголовками клиента Happ + HWID. Кэширует на диск;
    /// при сетевой ошибке отдаёт кэш (офлайн-режим).
    ///
    /// **Анти-SSRF:** `url` приходит от сервера (`key.subscription_url`). Прежде чем
    /// идти по нему, проверяем: схема https + хост принадлежит регистрируемому домену
    /// discovery (напр. `*.infinityconnect.ru`). Иначе скомпрометированный API мог бы
    /// заставить клиент сходить на произвольный/внутренний адрес.
    pub async fn subscription_body(&self, url: &str) -> AppResult<SubscriptionBody> {
        let trusted = self.trusted_domain().await;
        if !is_subscription_url_allowed(url, trusted.as_deref()) {
            return Err(AppError::Other("недоверенный URL подписки отклонён".into()));
        }
        let cache_name = store::subscription_cache_name(url);
        let resp = self
            .http
            .get(url)
            .header("User-Agent", device::USER_AGENT)
            .header("x-hwid", device::hwid())
            .header("x-device-os", device::device_os())
            .header("x-ver-os", device::os_version())
            .header("x-device-model", device::device_model())
            .send()
            .await;

        match resp {
            Ok(r) => {
                let interval = r
                    .headers()
                    .get("profile-update-interval")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .filter(|h| (1..=168).contains(h));
                let raw = r.error_for_status()?.text().await?;
                let body = SubscriptionBody { raw, update_interval_hours: interval };
                let _ = store::write_cache_secure(&cache_name, &body);
                Ok(body)
            }
            Err(e) => {
                // Офлайн: зашифрованный кэш тела подписки.
                store::read_cache_secure::<SubscriptionBody>(&cache_name)
                    .ok_or_else(|| AppError::from(e))
            }
        }
    }
}

/// Нормализует base_url: убирает хвостовой `/` (пути добавляем сами).
fn normalize_base(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

/// Регистрируемый домен (последние две метки: `sub.infinityconnect.ru` →
/// `infinityconnect.ru`). Для host из >=2 меток; иначе None.
fn registrable_domain(host: &str) -> Option<String> {
    let h = host.trim().trim_end_matches('.').to_lowercase();
    if h.is_empty() || h.parse::<std::net::IpAddr>().is_ok() {
        return None; // голый IP доменом не считаем
    }
    let labels: Vec<&str> = h.split('.').filter(|l| !l.is_empty()).collect();
    if labels.len() < 2 {
        return None;
    }
    Some(labels[labels.len() - 2..].join("."))
}

/// Разрешён ли URL подписки: только https + хост принадлежит доверенному
/// регистрируемому домену (сам домен или его поддомен). `trusted=None`
/// (discovery не было) → отклоняем на всякий случай.
fn is_subscription_url_allowed(url: &str, trusted: Option<&str>) -> bool {
    let Some(trusted) = trusted else { return false };
    // Схема должна быть https.
    let rest = match url.strip_prefix("https://") {
        Some(r) => r,
        None => return false,
    };
    // Хост — до первого '/', ':', '?', отбрасываем возможный user@ (anti-spoof).
    let host_part = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_part = host_part.rsplit('@').next().unwrap_or(host_part); // user@host → host
    let host = host_part.split(':').next().unwrap_or("").to_lowercase();
    match registrable_domain(&host) {
        Some(reg) => reg == trusted, // домен или его поддомен (reg одинаков)
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_subscription_url_allowed, registrable_domain};

    #[test]
    fn registrable_domain_extracts_last_two_labels() {
        assert_eq!(registrable_domain("sub.infinityconnect.ru").as_deref(), Some("infinityconnect.ru"));
        assert_eq!(registrable_domain("infinityconnect.ru").as_deref(), Some("infinityconnect.ru"));
        assert_eq!(registrable_domain("a.b.c.example.com").as_deref(), Some("example.com"));
        assert_eq!(registrable_domain("localhost"), None);
        assert_eq!(registrable_domain("169.254.169.254"), None);
    }

    #[test]
    fn allows_subdomain_of_trusted() {
        let t = Some("infinityconnect.ru");
        assert!(is_subscription_url_allowed("https://sub.infinityconnect.ru/abc", t));
        assert!(is_subscription_url_allowed("https://infinityconnect.ru/abc", t));
        assert!(is_subscription_url_allowed("https://sub.infinityconnect.ru:8443/abc", t));
    }

    #[test]
    fn blocks_ssrf_and_foreign_hosts() {
        let t = Some("infinityconnect.ru");
        // Внутренние/облачные метаданные.
        assert!(!is_subscription_url_allowed("http://169.254.169.254/latest/meta-data", t));
        assert!(!is_subscription_url_allowed("https://169.254.169.254/", t));
        assert!(!is_subscription_url_allowed("https://localhost/x", t));
        // Чужой домен.
        assert!(!is_subscription_url_allowed("https://evil.com/x", t));
        // Спуфинг через user@ (host = evil.com, не наш).
        assert!(!is_subscription_url_allowed("https://sub.infinityconnect.ru@evil.com/x", t));
        // Домен-обманка (суффикс совпадает, но регистрируемый домен другой).
        assert!(!is_subscription_url_allowed("https://infinityconnect.ru.evil.com/x", t));
        // Не-https.
        assert!(!is_subscription_url_allowed("http://sub.infinityconnect.ru/x", t));
        // Нет discovery.
        assert!(!is_subscription_url_allowed("https://sub.infinityconnect.ru/x", None));
    }
}
