//! Локальное хранилище: токены (DPAPI) + офлайн-кэши на `%APPDATA%\InfinityConnect\`
//! (аналог Android `TokenStorage`/`SettingsStore`/`SubscriptionCacheStore`).
//!
//! Офлайн-режим (как Happ/INCY): discovery, ключи и тела подписок переживают
//! перезапуск — connect строит конфиг из кэша без сети. См. memory `offline-mode`.

mod dpapi;

use std::fs;
use std::path::PathBuf;

use serde::{de::DeserializeOwned, Serialize};

use crate::error::{AppError, AppResult};

/// Пара токенов авторизации на диске (в зашифрованном виде).
#[derive(Debug, Clone, Serialize, serde::Deserialize, Default)]
pub struct Tokens {
    pub access: String,
    pub refresh: String,
}

/// Каталог данных приложения: `%APPDATA%\InfinityConnect\`.
pub fn data_dir() -> AppResult<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| AppError::Storage("не найден %APPDATA%".into()))?;
    let dir = base.join("InfinityConnect");
    fs::create_dir_all(&dir).map_err(|e| AppError::Storage(e.to_string()))?;
    Ok(dir)
}

fn path(name: &str) -> AppResult<PathBuf> {
    Ok(data_dir()?.join(name))
}

// ── Токены (DPAPI) ──

const TOKENS_FILE: &str = "tokens.bin";

/// Сохраняет токены зашифрованными DPAPI.
pub fn save_tokens(tokens: &Tokens) -> AppResult<()> {
    let json = serde_json::to_vec(tokens)?;
    let cipher = dpapi::protect(&json)?;
    fs::write(path(TOKENS_FILE)?, cipher).map_err(|e| AppError::Storage(e.to_string()))
}

/// Загружает токены (расшифровка DPAPI). `None`, если файла нет.
pub fn load_tokens() -> AppResult<Option<Tokens>> {
    let p = path(TOKENS_FILE)?;
    if !p.exists() {
        return Ok(None);
    }
    let cipher = fs::read(&p).map_err(|e| AppError::Storage(e.to_string()))?;
    let json = dpapi::unprotect(&cipher)?;
    let tokens = serde_json::from_slice(&json)?;
    Ok(Some(tokens))
}

/// Удаляет токены (разлогин).
pub fn clear_tokens() -> AppResult<()> {
    let p = path(TOKENS_FILE)?;
    if p.exists() {
        fs::remove_file(&p).map_err(|e| AppError::Storage(e.to_string()))?;
    }
    Ok(())
}

// ── Офлайн-кэши (обычный JSON) ──

/// Пишет значение как JSON-кэш (discovery/ключи/тела подписок).
pub fn write_cache<T: Serialize>(name: &str, value: &T) -> AppResult<()> {
    let json = serde_json::to_vec_pretty(value)?;
    fs::write(path(name)?, json).map_err(|e| AppError::Storage(e.to_string()))
}

/// Читает JSON-кэш. `None`, если файла нет или он повреждён.
pub fn read_cache<T: DeserializeOwned>(name: &str) -> Option<T> {
    let p = path(name).ok()?;
    let bytes = fs::read(p).ok()?;
    serde_json::from_slice(&bytes).ok()
}

// ── Зашифрованные кэши (DPAPI) для чувствительных данных ──
// Тела подписок и ключи содержат адреса серверов, UUID, Reality-ключи. На диске
// храним их зашифрованными (привязка к пользователю ОС), чтобы casual-доступ к
// файлам не раскрывал конфиги. NB: это защита ДИСКА, не от владельца машины —
// в памяти процесса и в running-конфиге данные всё равно есть.

/// Пишет значение как зашифрованный DPAPI-кэш (расширение .bin).
pub fn write_cache_secure<T: Serialize>(name: &str, value: &T) -> AppResult<()> {
    let json = serde_json::to_vec(value)?;
    let cipher = dpapi::protect(&json)?;
    fs::write(path(name)?, cipher).map_err(|e| AppError::Storage(e.to_string()))
}

/// Читает зашифрованный DPAPI-кэш. `None`, если нет/повреждён/чужой пользователь.
pub fn read_cache_secure<T: DeserializeOwned>(name: &str) -> Option<T> {
    let p = path(name).ok()?;
    let cipher = fs::read(p).ok()?;
    let json = dpapi::unprotect(&cipher).ok()?;
    serde_json::from_slice(&json).ok()
}

/// Имена файлов кэшей. Discovery — публичные URL, не секрет (JSON). Ключи —
/// содержат subscription_url/адреса, храним зашифрованными (.bin).
pub const CACHE_DISCOVERY: &str = "cache_discovery.json";
pub const CACHE_KEYS: &str = "cache_keys.bin";
/// Настройки пинга (JSON — не секрет).
pub const PING_SETTINGS: &str = "ping_settings.json";
/// Настройки маршрутизации (JSON — не секрет).
pub const ROUTING_SETTINGS: &str = "routing_settings.json";

/// Имя файла зашифрованного кэша тела подписки по её URL (хэш имени).
/// Тело подписки — самое чувствительное (адреса, UUID, Reality-ключи).
pub fn subscription_cache_name(url: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in url.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("cache_sub_{hash:016x}.bin")
}
