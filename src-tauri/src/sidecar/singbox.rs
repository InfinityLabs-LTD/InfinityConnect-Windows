//! Ядро sing-box как дочерний процесс: поднимает TUN и маршрутизирует трафик по
//! процессам (per-app split-tunnel), проксируя в локальный SOCKS ядра-прокси.
//! Статистику трафика читаем ЗДЕСЬ через Clash-API (`/connections` →
//! uploadTotal/downloadTotal): sing-box видит ВЕСЬ трафик, поэтому счётчик корректен
//! для любого ядра-прокси (у hysteria v2.10 собственный trafficStats не работает).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::engine::singbox_config::CLASH_API_PORT;
use crate::error::{AppError, AppResult};

use super::{hide_window, CoreProcess, Traffic};

pub struct SingboxProcess {
    child: Child,
    config_path: PathBuf,
}

impl SingboxProcess {
    pub fn start(exe_dir: &Path, config_json: &str) -> AppResult<Self> {
        let exe = exe_dir.join("sing-box.exe");
        if !exe.exists() {
            return Err(AppError::Other(format!("sing-box.exe не найден: {}", exe.display())));
        }
        let config_path = exe_dir.join("running_singbox.json");
        let mut f = std::fs::File::create(&config_path)
            .map_err(|e| AppError::Other(format!("создание singbox-конфига: {e}")))?;
        f.write_all(config_json.as_bytes())
            .map_err(|e| AppError::Other(format!("запись singbox-конфига: {e}")))?;
        drop(f);

        let mut cmd = Command::new(&exe);
        cmd.arg("run")
            .arg("-c")
            .arg(&config_path)
            // cwd = каталог ядра: wintun.dll берётся отсюда.
            .current_dir(exe_dir)
            .stdout(Stdio::null())
            .stderr(super::core_log(exe_dir, "singbox"));
        hide_window(&mut cmd);

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Other(format!("запуск sing-box.exe: {e}")))?;
        Ok(Self { child, config_path })
    }
}

impl CoreProcess for SingboxProcess {
    fn exit_status(&mut self) -> Option<i32> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            _ => None,
        }
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.config_path);
    }

    fn query_traffic(&self) -> Traffic {
        clash_traffic(CLASH_API_PORT).unwrap_or_default()
    }
}

/// Читает накопленный трафик из Clash-API sing-box: GET /connections →
/// `{"uploadTotal":N,"downloadTotal":N,...}`. Минимальный HTTP без внешних зависимостей.
fn clash_traffic(port: u16) -> Option<Traffic> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).ok()?;

    let req = format!(
        "GET /connections HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp).ok()?;

    // Тело — после пустой строки (конец заголовков).
    let body = resp.split("\r\n\r\n").nth(1)?;
    let v: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    Some(Traffic {
        uplink: v.get("uploadTotal").and_then(|x| x.as_u64()).unwrap_or(0),
        downlink: v.get("downloadTotal").and_then(|x| x.as_u64()).unwrap_or(0),
    })
}

impl Drop for SingboxProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
