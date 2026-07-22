//! Мост фронт↔бэк: `#[tauri::command]`-функции, вызываемые из фронта через invoke().
//!
//! Фаза 1: аккаунт и подписки — discovery, login/logout, список ключей и серверов.
//! Туннеля ещё нет (Фаза 2). Все команды возвращают `Result<_, AppError>` —
//! ошибка сериализуется во фронт как `{kind, message}`.

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::api::dto::{DiscoveryDto, KeyDto, UserInfoDto};
use crate::api::ApiClient;
use crate::connection::build_connection;
use crate::engine::{selector, xray_config};
use crate::error::AppResult;
use crate::ping::model::PingSettings;
use crate::ping::Pinger;
use crate::routing::RoutingSettings;
use crate::store;
use crate::subscription;
use crate::tunnel::TunnelManager;

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

/// Подключение к серверу ключа по индексу (Фаза 2: только VLESS/RawXray → Xray).
/// Строит EngineConfig (подписка → fallback /v1/config), генерирует Xray JSON,
/// поднимает туннель. Состояние/статистика идут событиями.
#[tauri::command]
pub async fn connect(
    key_id: i64,
    server_index: usize,
    app: AppHandle,
    api: State<'_, ApiClient>,
    tunnel: State<'_, TunnelManager>,
) -> AppResult<()> {
    let config = build_connection(&api, key_id, server_index).await?;
    let routing = load_routing_settings();
    // Гибрид: ядро-прокси (socks) + sing-box (TUN + split-tunnel по процессам).
    // Split-tunnel по приложениям (3 режима) встроен в sing-box-конфиг через select().
    let plan = selector::select(&config, xray_config::DEFAULT_MTU, &routing);
    tunnel.connect(app, plan, routing.kill_switch).await
}

/// Отключение туннеля.
#[tauri::command]
pub async fn disconnect(app: AppHandle, tunnel: State<'_, TunnelManager>) -> AppResult<()> {
    tunnel.disconnect(&app).await;
    Ok(())
}

/// Текущий статус туннеля (подключён ли) — для восстановления UI.
#[tauri::command]
pub async fn tunnel_status(tunnel: State<'_, TunnelManager>) -> AppResult<bool> {
    Ok(tunnel.is_connected().await)
}

/// Включён ли автозапуск с ОС.
#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> AppResult<bool> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch()
        .is_enabled()
        .map_err(|e| crate::error::AppError::Other(format!("автозапуск: {e}")))
}

/// Включает/выключает автозапуск с ОС.
#[tauri::command]
pub fn set_autostart(enabled: bool, app: AppHandle) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    let res = if enabled { mgr.enable() } else { mgr.disable() };
    res.map_err(|e| crate::error::AppError::Other(format!("автозапуск: {e}")))
}

/// Пинг сервера ключа текущим методом (из настроек). Возвращает мс или -1.
/// Строит профиль (подписка → fallback) и меряет в блокирующем пуле.
#[tauri::command]
pub async fn ping_server(
    key_id: i64,
    server_index: usize,
    api: State<'_, ApiClient>,
    pinger: State<'_, Pinger>,
) -> AppResult<i32> {
    let config = build_connection(&api, key_id, server_index).await?;
    let settings = load_ping_settings();
    let pinger = pinger.inner().clone();
    // Пинг блокирующий (сокеты/процесс) — уводим из async-рантайма.
    let ms = tauri::async_runtime::spawn_blocking(move || pinger.measure(&config, &settings))
        .await
        .unwrap_or(-1);
    Ok(ms)
}

/// Текущие настройки пинга (из кэша или дефолт).
#[tauri::command]
pub fn get_ping_settings() -> AppResult<PingSettings> {
    Ok(load_ping_settings())
}

/// Сохраняет настройки пинга.
#[tauri::command]
pub fn set_ping_settings(settings: PingSettings) -> AppResult<()> {
    store::write_cache(store::PING_SETTINGS, &settings)
}

fn load_ping_settings() -> PingSettings {
    store::read_cache::<PingSettings>(store::PING_SETTINGS).unwrap_or_default()
}

/// Текущие настройки маршрутизации.
#[tauri::command]
pub fn get_routing_settings() -> AppResult<RoutingSettings> {
    Ok(load_routing_settings())
}

/// Сохраняет настройки маршрутизации. Применятся при следующем connect.
#[tauri::command]
pub fn set_routing_settings(settings: RoutingSettings) -> AppResult<()> {
    store::write_cache(store::ROUTING_SETTINGS, &settings)
}

fn load_routing_settings() -> RoutingSettings {
    store::read_cache::<RoutingSettings>(store::ROUTING_SETTINGS).unwrap_or_default()
}

/// Список установленных приложений (для выбора в split-tunnel). Сканирование
/// Start Menu блокирующее — уводим в blocking-пул.
#[tauri::command]
pub async fn list_installed_apps() -> AppResult<Vec<crate::apps::InstalledApp>> {
    let apps = tauri::async_runtime::spawn_blocking(crate::apps::list_installed)
        .await
        .unwrap_or_default();
    Ok(apps)
}

/// Каталог с логами ядер (`<bin>/<core>_stderr.log`).
pub struct LogsDir(pub std::path::PathBuf);

/// Лог одного ядра для экрана логов.
#[derive(Debug, Serialize)]
pub struct CoreLog {
    /// Ядро: "xray" | "hysteria" | "singbox".
    pub core: String,
    /// Содержимое stderr-лога (может быть пустым).
    pub content: String,
}

/// Читает stderr-логи всех ядер (для экрана логов в настройках).
#[tauri::command]
pub fn read_core_logs(logs: State<'_, LogsDir>) -> AppResult<Vec<CoreLog>> {
    let dir = logs.0.clone();
    let mut out = Vec::new();
    for core in ["singbox", "xray", "hysteria"] {
        let path = dir.join(format!("{core}_stderr.log"));
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        out.push(CoreLog { core: core.to_string(), content });
    }
    Ok(out)
}

/// Очищает stderr-логи ядер (кнопка «Очистить» на экране логов).
#[tauri::command]
pub fn clear_core_logs(logs: State<'_, LogsDir>) -> AppResult<()> {
    let dir = logs.0.clone();
    for core in ["singbox", "xray", "hysteria"] {
        let _ = std::fs::write(dir.join(format!("{core}_stderr.log")), b"");
    }
    Ok(())
}
