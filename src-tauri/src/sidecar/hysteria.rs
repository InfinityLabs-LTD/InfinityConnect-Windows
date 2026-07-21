//! Ядро Hysteria2 как дочерний процесс (аналог Android `Hysteria2CoreBridge`).
//! Ядро само поднимает wintun через секцию `tun`. Статистика — через HTTP API
//! `trafficStats` (GET /traffic → {"tx":bytes,"rx":bytes}).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::error::{AppError, AppResult};

use super::{hide_window, CoreProcess, Traffic};

pub struct HysteriaProcess {
    child: Child,
    config_path: PathBuf,
    stats_port: u16,
}

impl HysteriaProcess {
    pub fn start(exe_dir: &Path, config_json: &str, stats_port: u16) -> AppResult<Self> {
        let exe = exe_dir.join("hysteria.exe");
        if !exe.exists() {
            return Err(AppError::Other(format!("hysteria.exe не найден: {}", exe.display())));
        }
        let config_path = exe_dir.join("running_hysteria.json");
        let mut f = std::fs::File::create(&config_path)
            .map_err(|e| AppError::Other(format!("создание конфига: {e}")))?;
        f.write_all(config_json.as_bytes())
            .map_err(|e| AppError::Other(format!("запись конфига: {e}")))?;
        drop(f);

        let mut cmd = Command::new(&exe);
        cmd.arg("client")
            .arg("-c")
            .arg(&config_path)
            .arg("--disable-update-check")
            .current_dir(exe_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_window(&mut cmd);

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Other(format!("запуск hysteria.exe: {e}")))?;
        Ok(Self { child, config_path, stats_port })
    }
}

impl CoreProcess for HysteriaProcess {
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
        http_traffic(self.stats_port).unwrap_or_default()
    }
}

impl Drop for HysteriaProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Минимальный HTTP GET к локальному trafficStats API (без внешних зависимостей).
/// Парсит `{"tx":N,"rx":N}` (tx=uplink, rx=downlink).
fn http_traffic(port: u16) -> Option<Traffic> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).ok()?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).ok()?;

    let req = format!(
        "GET /traffic HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).ok()?;

    let mut resp = String::new();
    stream.read_to_string(&mut resp).ok()?;

    // Тело — после пустой строки (конец заголовков).
    let body = resp.split("\r\n\r\n").nth(1)?;
    let v: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    Some(Traffic {
        uplink: v.get("tx").and_then(|x| x.as_u64()).unwrap_or(0),
        downlink: v.get("rx").and_then(|x| x.as_u64()).unwrap_or(0),
    })
}
