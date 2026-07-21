//! Настройки маршрутизации (аналог Android `domain/model/Routing.kt`).
//!
//! Два уровня:
//!  - **по сайтам** (домены → Xray `routing.rules`) — работает для Xray-ядра
//!    (VLESS/RawXray). Переносится по смыслу почти даром.
//!  - **по приложениям** (split-tunnel по процессу) — на Windows реализуется
//!    через WFP; нативного аналога Android allow/disallow нет. На первой итерации
//!    (Фаза 6) — модель есть, реальный WFP-фильтр — задел (`apply_per_app`).
//!
//! Общий режим трафика = ALL (весь трафик в VPN, приватные сети direct), как в
//! текущем Android-UI. BYPASS_RU/CUSTOM в UI убраны — оставляем ALL.

pub mod perapp;

use serde::{Deserialize, Serialize};

/// Режим маршрутизации по сайтам (доменам) — правила routing.rules Xray.
/// Для Hysteria2 доменные правила из UI не применяются.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SiteRoutingMode {
    /// Список доменов не используется.
    #[default]
    Off,
    /// Указанные домены — через VPN (proxy).
    Proxy,
    /// Указанные домены — напрямую (direct).
    Direct,
}

/// Режим фильтрации по приложениям (split-tunnel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppRoutingMode {
    /// Не фильтровать по приложениям.
    #[default]
    Off,
    /// Через VPN идут ТОЛЬКО выбранные приложения.
    Allow,
    /// Через VPN идёт всё, КРОМЕ выбранных.
    Disallow,
}

/// Настройки маршрутизации.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingSettings {
    pub site_mode: SiteRoutingMode,
    /// Домены для site_mode (например "youtube.com" — матчит и поддомены).
    pub sites: Vec<String>,
    pub app_mode: AppRoutingMode,
    /// Пути/имена процессов для app_mode (например "chrome.exe").
    pub apps: Vec<String>,
}
