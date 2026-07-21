//! Ядро Xray как дочерний процесс (аналог Android `XrayCoreBridge`, но процесс).
//! Ядро само поднимает wintun через tun-инбаунд. Статистика — через встроенный
//! CLI `xray api statsquery` (gRPC StatsService на локальном api-порту).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::error::{AppError, AppResult};

use super::{hide_window, CoreProcess, Traffic};

pub struct XrayProcess {
    child: Child,
    exe: PathBuf,
    config_path: PathBuf,
    stats_port: u16,
}

impl XrayProcess {
    pub fn start(exe_dir: &Path, config_json: &str, stats_port: u16) -> AppResult<Self> {
        let exe = exe_dir.join("xray.exe");
        if !exe.exists() {
            return Err(AppError::Other(format!("xray.exe не найден: {}", exe.display())));
        }
        let config_path = exe_dir.join("running_xray.json");
        write_config(&config_path, config_json)?;

        let mut cmd = Command::new(&exe);
        cmd.arg("run")
            .arg("-config")
            .arg(&config_path)
            // cwd = каталог ядра: wintun.dll и geo-файлы берутся отсюда.
            .current_dir(exe_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_window(&mut cmd);

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Other(format!("запуск xray.exe: {e}")))?;
        Ok(Self { child, exe, config_path, stats_port })
    }
}

impl CoreProcess for XrayProcess {
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
        let server = format!("127.0.0.1:{}", self.stats_port);
        let mut cmd = Command::new(&self.exe);
        cmd.arg("api")
            .arg("statsquery")
            .arg(format!("--server={server}"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_window(&mut cmd);

        let Ok(out) = cmd.output() else {
            return Traffic::default();
        };
        parse_stats(&String::from_utf8_lossy(&out.stdout))
    }
}

impl Drop for XrayProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

fn write_config(path: &Path, json: &str) -> AppResult<()> {
    let mut f = std::fs::File::create(path)
        .map_err(|e| AppError::Other(format!("создание конфига: {e}")))?;
    f.write_all(json.as_bytes())
        .map_err(|e| AppError::Other(format!("запись конфига: {e}")))?;
    Ok(())
}

/// Суммирует все `...>>>uplink` и `...>>>downlink` из `xray api statsquery`.
fn parse_stats(json: &str) -> Traffic {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Traffic::default();
    };
    let mut t = Traffic::default();
    if let Some(stats) = v.get("stat").and_then(|s| s.as_array()) {
        for item in stats {
            let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let value = item
                .get("value")
                .and_then(|v| v.as_str().and_then(|s| s.parse::<u64>().ok()).or_else(|| v.as_u64()))
                .unwrap_or(0);
            if name.ends_with(">>>uplink") {
                t.uplink += value;
            } else if name.ends_with(">>>downlink") {
                t.downlink += value;
            }
        }
    }
    t
}
