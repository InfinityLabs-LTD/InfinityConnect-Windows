//! Модель измерения пинга (аналог Android `domain/model/Ping.kt`).
//! Схема как в Happ: 4 метода + 3 режима (для proxy) + таймаут.

use serde::{Deserialize, Serialize};

/// Протокол измерения пинга. Цвет пилла в UI — по КАЧЕСТВУ, не по методу.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PingMethod {
    /// HTTP GET к тест-URL ЧЕРЕЗ протокол сервера (локальный SOCKS-inbound ядра).
    /// End-to-end задержка через VLESS/Reality. Для Hysteria2 — откат на TCP.
    /// ДЕФОЛТ: TCP-хендшейк до этих серверов терминируется локально/на edge (~1мс,
    /// бесполезен как метрика) — реальный RTT даёт только сквозной proxy-пинг.
    #[default]
    ProxyGet,
    /// HTTP HEAD через прокси-сервер: только заголовки, легче GET.
    ProxyHead,
    /// TCP-хендшейк до host:port: чистая задержка до узла.
    Tcp,
    /// ICMP echo до адреса сервера: сетевой RTT без TCP/TLS.
    Icmp,
}

/// Режим proxy-пинга (via …). Действует только для proxy-методов.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PingMode {
    /// Несколько независимых запросов, берётся лучший (минимум).
    #[default]
    Default,
    /// Два запроса на новых соединениях; меряется второй (первый — прогрев).
    Double,
    /// Два запроса по одному TLS-соединению; меряется второй (без TLS-хендшейка).
    Keepalive,
}

/// Настройки измерения пинга.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingSettings {
    pub method: PingMethod,
    pub mode: PingMode,
    pub test_url: String,
    /// Таймаут proxy-пинга, сек (зажимается в 5..15).
    pub timeout_sec: u32,
}

impl Default for PingSettings {
    fn default() -> Self {
        Self {
            method: PingMethod::default(),
            mode: PingMode::default(),
            test_url: DEFAULT_TEST_URL.to_string(),
            timeout_sec: DEFAULT_TIMEOUT_SEC,
        }
    }
}

impl PingSettings {
    /// Таймаут в мс с зажимом в допустимый диапазон.
    pub fn timeout_ms(&self) -> u64 {
        (self.timeout_sec.clamp(MIN_TIMEOUT_SEC, MAX_TIMEOUT_SEC) as u64) * 1000
    }
}

/// Быстрый эндпоинт, отдающий 204 без тела (как в Happ).
pub const DEFAULT_TEST_URL: &str = "https://www.gstatic.com/generate_204";
pub const MIN_TIMEOUT_SEC: u32 = 5;
pub const MAX_TIMEOUT_SEC: u32 = 15;
pub const DEFAULT_TIMEOUT_SEC: u32 = 7;
