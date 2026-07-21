//! Модель профиля сервера (аналог Android `domain/engine/EngineConfig.kt`).
//!
//! `EngineConfig` — единственный тип-мост профиля между разбором подписки и
//! движком. Парсер подписки (`subscription/`) преобразует VLESS/hy2-URI и
//! JSON-конфиги панели в один из вариантов ниже. Движок выбирается по варианту:
//! Vless/RawXray → Xray, Hysteria2 → Hysteria2.
//!
//! Сборка Xray-JSON из этой модели — `engine/xray_config.rs`.

pub mod xray_config;

use serde::Serialize;
use serde_json::Value;

/// Разобранный профиль одного сервера подписки — вход для VPN-движка.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "protocol")]
pub enum EngineConfig {
    /// Профиль VLESS (в т.ч. Reality и транспорт XHTTP) — движок Xray.
    Vless(VlessConfig),

    /// Готовый Xray-конфиг из подписки, пробрасываемый в ядро почти как есть.
    /// «Сложные» серверы панели (Remnawave) с несколькими outbounds, routing и
    /// balancers (автовыбор «LTE | Все операторы»: balancer MAIN → скрытый WHITE).
    /// Схлопывать в один outbound нельзя — потеряется маршрутизация и fallback.
    RawXray(RawXrayConfig),

    /// Профиль Hysteria2 — движок Hysteria2.
    Hysteria2(Hysteria2Config),
}

impl EngineConfig {
    /// Отображаемое имя сервера (remark из URI / имя из API).
    pub fn remark(&self) -> &str {
        match self {
            EngineConfig::Vless(c) => &c.remark,
            EngineConfig::RawXray(c) => &c.remark,
            EngineConfig::Hysteria2(c) => &c.remark,
        }
    }

    /// Адрес для UI/пинга. У RawXray берётся из primary_outbound (заглушка «—»).
    pub fn address(&self) -> &str {
        match self {
            EngineConfig::Vless(c) => &c.address,
            EngineConfig::Hysteria2(c) => &c.address,
            EngineConfig::RawXray(c) => {
                c.primary_outbound.as_ref().map(|p| p.address.as_str()).unwrap_or("—")
            }
        }
    }

    /// Порт для UI/пинга. У RawXray — из primary_outbound (иначе 0).
    pub fn port(&self) -> u16 {
        match self {
            EngineConfig::Vless(c) => c.port,
            EngineConfig::Hysteria2(c) => c.port,
            EngineConfig::RawXray(c) => c.primary_outbound.as_ref().map(|p| p.port).unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VlessConfig {
    pub remark: String,
    pub address: String,
    pub port: u16,
    pub uuid: String,
    pub transport: Transport,
    pub security: Security,
    /// Например `xtls-rprx-vision`; пусто для не-tcp транспортов.
    pub flow: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RawXrayConfig {
    pub remark: String,
    /// Корневой объект конфига (dns/routing/outbounds/…), как пришёл в подписке.
    pub root: Value,
    /// Первый proxy-outbound (для тест-пинга: balancer пинговать бессмысленно).
    pub primary_outbound: Option<VlessConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hysteria2Config {
    pub remark: String,
    pub address: String,
    pub port: u16,
    pub auth: String,
    pub sni: Option<String>,
    pub insecure: bool,
    pub obfs_password: Option<String>,
    pub up_mbps: Option<i32>,
    pub down_mbps: Option<i32>,
}

/// Транспорт VLESS.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Transport {
    /// TCP без надстроек (raw).
    Tcp,
    /// WebSocket.
    Ws { path: Option<String>, host: Option<String> },
    /// gRPC.
    Grpc { service_name: Option<String> },
    /// XHTTP (SplitHTTP). `extra` — сырой объект из xhttpSettings подписки/URI
    /// (xmux/xPadding/session/seq/…). Клиент его НЕ интерпретирует, пробрасывает
    /// в ядро целиком — иначе Xray-сервер белых списков отвергает соединение.
    Xhttp {
        path: Option<String>,
        host: Option<String>,
        /// auto | packet-up | stream-up | stream-one
        mode: Option<String>,
        extra: Option<Value>,
    },
}

/// Слой безопасности VLESS.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "security", rename_all = "lowercase")]
pub enum Security {
    /// Без TLS (обычно только для отладки).
    None,
    /// Классический TLS.
    Tls {
        sni: Option<String>,
        fingerprint: Option<String>,
        alpn: Option<Vec<String>>,
        allow_insecure: bool,
    },
    /// Reality.
    Reality {
        sni: Option<String>,
        fingerprint: Option<String>,
        public_key: String,
        short_id: Option<String>,
        spider_x: Option<String>,
    },
}
