//! Измерение задержки до сервера (аналог Android `PingServerUseCase` +
//! `XrayProxyPinger`). 4 метода как в Happ; значение всегда в мс, -1 при
//! недоступности/таймауте.
//!
//! - Proxy GET/HEAD — HTTP через локальный SOCKS-inbound временного xray-ядра
//!   (end-to-end через VLESS/Reality). Для не-VLESS — откат на TCP.
//! - TCP — время хендшейка host:port (медиана из замеров).
//! - ICMP — echo через IP Helper API.
//!
//! Замеры proxy-пинга сериализованы (один временный xray за раз): параллельные
//! процессы конфликтуют за wintun/порты не мешают, но экономим ресурсы и порты.

mod icmp;
pub mod model;
mod proxy;

use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::engine::EngineConfig;

use model::{PingMethod, PingSettings};

/// Пингер: держит каталог ядра (для временного xray SOCKS-sidecar) и лок
/// сериализации proxy-замеров. Клонируется дёшево (Arc внутри).
#[derive(Clone)]
pub struct Pinger {
    inner: Arc<PingerInner>,
}

struct PingerInner {
    exe_dir: PathBuf,
    proxy_lock: Mutex<()>,
}

impl Pinger {
    pub fn new(exe_dir: PathBuf) -> Self {
        Self { inner: Arc::new(PingerInner { exe_dir, proxy_lock: Mutex::new(()) }) }
    }

    /// Измеряет задержку до сервера выбранным методом. `config` нужен proxy-методам
    /// (меряют через ядро по профилю). Возвращает мс или -1.
    pub fn measure(&self, config: &EngineConfig, settings: &PingSettings) -> i32 {
        match settings.method {
            PingMethod::ProxyGet => self.proxy_ping(config, settings, false),
            PingMethod::ProxyHead => self.proxy_ping(config, settings, true),
            PingMethod::Tcp => tcp_ping(config.address(), config.port()),
            PingMethod::Icmp => icmp::ping(config.address()),
        }
    }

    /// Proxy-пинг через временное xray-ядро. Только VLESS; иначе откат на TCP.
    fn proxy_ping(&self, config: &EngineConfig, settings: &PingSettings, head: bool) -> i32 {
        let vless = match config {
            EngineConfig::Vless(v) => v,
            // Для RawXray меряем основной сервер (MAIN).
            EngineConfig::RawXray(r) => match &r.primary_outbound {
                Some(v) => v,
                None => return tcp_ping(config.address(), config.port()),
            },
            // Hysteria2 — другое ядро; откат на TCP.
            EngineConfig::Hysteria2(_) => return tcp_ping(config.address(), config.port()),
        };
        let _guard = self.inner.proxy_lock.lock().unwrap();
        let ms = proxy::measure(&self.inner.exe_dir, vless, settings, head);
        if ms >= 0 {
            ms
        } else {
            tcp_ping(config.address(), config.port())
        }
    }
}

/// TCP-хендшейк host:port, устойчивый к выбросам: 1 прогрев + 4 замера, медиана.
fn tcp_ping(address: &str, port: u16) -> i32 {
    // Резолвим заранее, чтобы DNS не попадал в измерение.
    let Some(addr) = (address, port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut it| it.next())
    else {
        return -1;
    };

    let mut samples: Vec<i32> = Vec::with_capacity(TCP_ATTEMPTS);
    for attempt in 0..(TCP_WARMUP + TCP_ATTEMPTS) {
        let start = Instant::now();
        let ok = TcpStream::connect_timeout(&addr, Duration::from_millis(TIMEOUT_MS)).is_ok();
        let ms = if ok { start.elapsed().as_millis() as i32 } else { -1 };
        if attempt >= TCP_WARMUP && ms >= 0 {
            samples.push(ms);
        }
    }
    median(&mut samples)
}

fn median(samples: &mut [i32]) -> i32 {
    if samples.is_empty() {
        return -1;
    }
    samples.sort_unstable();
    let mid = samples.len() / 2;
    if samples.len() % 2 == 1 {
        samples[mid]
    } else {
        (samples[mid - 1] + samples[mid]) / 2
    }
}

const TIMEOUT_MS: u64 = 6000;
const TCP_WARMUP: usize = 1;
const TCP_ATTEMPTS: usize = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_even_empty() {
        assert_eq!(median(&mut [30, 10, 20]), 20);
        assert_eq!(median(&mut [10, 20, 30, 40]), 25);
        assert_eq!(median(&mut []), -1);
    }
}
