//! Единый тип ошибок приложения (аналог Android `AppError`/`AppResult`).
//! Все слои (api/subscription/engine/store) возвращают `Result<T, AppError>`.
//! Ошибка сериализуема — её можно вернуть во фронт из `#[tauri::command]`.

use serde::Serialize;

/// Ошибка приложения. `kind` даёт фронту машиночитаемую категорию,
/// `message` — человекочитаемый текст (на русском, для UI).
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    /// Сеть/HTTP: недоступность, таймаут, не-2xx статус.
    #[error("Сеть: {0}")]
    Network(String),

    /// Не авторизован (401 после неудачного refresh) — нужен повторный логин.
    #[error("Требуется авторизация")]
    Unauthorized,

    /// Разбор ответа/подписки/URI.
    #[error("Разбор: {0}")]
    Parse(String),

    /// Локальное хранилище (DPAPI, файлы, реестр).
    #[error("Хранилище: {0}")]
    Storage(String),

    /// Прочее.
    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AppError::Network(format!("таймаут: {e}"))
        } else if e.is_status() && e.status() == Some(reqwest::StatusCode::UNAUTHORIZED) {
            AppError::Unauthorized
        } else {
            AppError::Network(e.to_string())
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Parse(e.to_string())
    }
}
