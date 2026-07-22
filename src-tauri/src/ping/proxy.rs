//! Proxy-пинг через временное xray-ядро (аналог Android `XrayProxyPinger`).
//! Поднимает xray.exe с локальным SOCKS-inbound по профилю (без TUN), гонит
//! HTTP через этот SOCKS выбранным методом (GET/HEAD) и режимом. RTT мс или -1.

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;

use crate::engine::{xray_config, VlessConfig};

use super::model::{PingMode, PingSettings};

/// Максимум ожидания готовности SOCKS-inbound ядра перед первым запросом.
/// Порт открывается почти сразу, но VLESS-outbound к серверу — не мгновенно;
/// ждём именно принятия соединения на SOCKS-порту, а не фиксированную паузу.
const CORE_READY_TIMEOUT: Duration = Duration::from_millis(4000);
const CORE_POLL: Duration = Duration::from_millis(50);
/// Число попыток в режиме Default (берём лучшую).
const DEFAULT_ATTEMPTS: usize = 3;

/// Меряет задержку до сервера через его протокол. -1 при ошибке.
pub fn measure(exe_dir: &Path, config: &VlessConfig, settings: &PingSettings, head: bool) -> i32 {
    let exe = exe_dir.join("xray.exe");
    if !exe.exists() {
        return -1;
    }
    let Some(port) = free_port() else { return -1 };

    // Отдельный временный конфиг (не конфликтует с running_xray.json туннеля).
    let config_json = xray_config::build_proxy_ping(config, port);
    let config_path = exe_dir.join(format!("ping_{port}.json"));
    if std::fs::File::create(&config_path)
        .and_then(|mut f| f.write_all(config_json.as_bytes()))
        .is_err()
    {
        return -1;
    }

    let mut child = match spawn_core(&exe, exe_dir, &config_path) {
        Ok(c) => c,
        Err(_) => {
            let _ = std::fs::remove_file(&config_path);
            return -1;
        }
    };

    // Ждём, пока SOCKS-inbound начнёт принимать соединения (порт открыт),
    // затем даём короткий запас на установку outbound к серверу.
    let ready = wait_socks_ready(port);

    let result = if ready {
        request_through_proxy(port, settings, head)
    } else {
        -1
    };

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&config_path);
    result
}

/// Ждёт, пока ядро начнёт слушать SOCKS-порт (TCP connect успешен).
/// Возвращает false, если за таймаут порт так и не открылся.
fn wait_socks_ready(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let deadline = Instant::now() + CORE_READY_TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(sa) = addr.parse() {
            if TcpStream::connect_timeout(&sa, CORE_POLL).is_ok() {
                // Порт принимает соединения. Небольшой запас на прогрев маршрута.
                std::thread::sleep(Duration::from_millis(120));
                return true;
            }
        }
        std::thread::sleep(CORE_POLL);
    }
    false
}

fn spawn_core(exe: &Path, exe_dir: &Path, config_path: &Path) -> std::io::Result<Child> {
    let mut cmd = Command::new(exe);
    cmd.arg("run")
        .arg("-config")
        .arg(config_path)
        .current_dir(exe_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.spawn()
}

/// Гонит HTTP через SOCKS-прокси ядра выбранным режимом; мс или -1.
fn request_through_proxy(port: u16, settings: &PingSettings, head: bool) -> i32 {
    let timeout = Duration::from_millis(settings.timeout_ms());
    let Ok(proxy) = reqwest::Proxy::all(format!("socks5://127.0.0.1:{port}")) else {
        return -1;
    };
    let Ok(client) = Client::builder()
        .proxy(proxy)
        .connect_timeout(timeout)
        .timeout(timeout)
        .build()
    else {
        return -1;
    };

    match settings.mode {
        // Несколько независимых запросов — берём лучший (минимум).
        PingMode::Default => {
            let mut best = -1;
            for _ in 0..DEFAULT_ATTEMPTS {
                let ms = single(&client, &settings.test_url, head);
                if ms >= 0 && (best < 0 || ms < best) {
                    best = ms;
                }
            }
            best
        }
        // Два запроса; меряем второй (первый — прогрев ядра/маршрута).
        PingMode::Double => {
            let _ = single(&client, &settings.test_url, head);
            single(&client, &settings.test_url, head)
        }
        // Два запроса по переиспользуемому пулу; меряем второй (без нового хендшейка).
        PingMode::Keepalive => {
            let first = single(&client, &settings.test_url, head);
            if first < 0 {
                -1
            } else {
                single(&client, &settings.test_url, head)
            }
        }
    }
}

/// Один HTTP-запрос через прокси; RTT до конца ответа в мс, либо -1.
/// Меряем в микросекундах и округляем вверх до 1 мс: реальный сетевой RTT
/// не бывает 0 мс, а `as_millis()` субмиллисекундные значения обнулял.
fn single(client: &Client, url: &str, head: bool) -> i32 {
    let req = if head { client.head(url) } else { client.get(url) };
    let start = Instant::now();
    match req.header("Cache-Control", "no-cache").send() {
        Ok(resp) => {
            let code = resp.status().as_u16();
            // Дочитываем тело (для GET RTT включает полный ответ).
            let _ = resp.bytes();
            if (200..400).contains(&code) {
                us_to_ms(start.elapsed())
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Длительность → мс, округление вверх, минимум 1 (0 мс невозможно для сети).
pub(super) fn us_to_ms(d: Duration) -> i32 {
    let us = d.as_micros();
    (((us + 999) / 1000) as i32).max(1)
}

/// Свободный локальный TCP-порт для SOCKS-inbound ядра.
fn free_port() -> Option<u16> {
    TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok().map(|a| a.port())
}
