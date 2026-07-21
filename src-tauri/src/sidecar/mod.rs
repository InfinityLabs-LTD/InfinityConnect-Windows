//! Запуск и менеджмент ядра `xray.exe` как дочернего процесса
//! (аналог Android `XrayCoreBridge`, но здесь — отдельный процесс, не JNI).
//!
//! Ядро само поднимает wintun-адаптер через tun-инбаунд (см. `engine::xray_config`).
//! Статистику трафика читаем через встроенный CLI `xray api statsquery` (gRPC
//! StatsService на локальном api-порту) — не тащим tonic в зависимости.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::engine::xray_config::STATS_API_PORT;
use crate::error::{AppError, AppResult};

/// Скрывает окно консоли дочернего процесса на Windows.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Управляемый инстанс ядра Xray.
pub struct XrayProcess {
    child: Child,
    exe: PathBuf,
    /// Временный файл конфига (удаляется при остановке).
    config_path: PathBuf,
}

impl XrayProcess {
    /// Запускает `xray.exe run -config <tmp>`. `exe_dir` — каталог с xray.exe,
    /// wintun.dll и geo-файлами (ядро ищет wintun.dll рядом с собой / в cwd).
    pub fn start(exe_dir: &Path, config_json: &str) -> AppResult<Self> {
        let exe = exe_dir.join("xray.exe");
        if !exe.exists() {
            return Err(AppError::Other(format!("xray.exe не найден: {}", exe.display())));
        }

        // Пишем конфиг во временный файл рядом с exe (там же geoip/geosite).
        let config_path = exe_dir.join("running_config.json");
        let mut f = std::fs::File::create(&config_path)
            .map_err(|e| AppError::Other(format!("создание конфига: {e}")))?;
        f.write_all(config_json.as_bytes())
            .map_err(|e| AppError::Other(format!("запись конфига: {e}")))?;
        drop(f);

        let mut cmd = Command::new(&exe);
        cmd.arg("run")
            .arg("-config")
            .arg(&config_path)
            // cwd = каталог ядра: wintun.dll и geo-файлы берутся отсюда.
            .current_dir(exe_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Other(format!("запуск xray.exe: {e}")))?;

        Ok(Self { child, exe, config_path })
    }

    /// Проверяет, жив ли процесс. `Some(code)` — если уже завершился.
    pub fn exit_status(&mut self) -> Option<i32> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            _ => None,
        }
    }

    /// Останавливает ядро и удаляет временный конфиг.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.config_path);
    }

    /// Суммарная статистика трафика (uplink+downlink по всем аутбаундам).
    /// Читает через `xray api statsquery`; при недоступности — нули.
    pub fn query_traffic(&self) -> Traffic {
        query_traffic(&self.exe)
    }
}

impl Drop for XrayProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Счётчики трафика (байты) с момента старта ядра.
#[derive(Debug, Clone, Copy, Default)]
pub struct Traffic {
    pub uplink: u64,
    pub downlink: u64,
}

/// Запрашивает статистику у ядра через встроенный CLI StatsService.
fn query_traffic(exe: &Path) -> Traffic {
    let server = format!("127.0.0.1:{STATS_API_PORT}");
    let mut cmd = Command::new(exe);
    cmd.arg("api")
        .arg("statsquery")
        .arg(format!("--server={server}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let Ok(out) = cmd.output() else {
        return Traffic::default();
    };
    parse_stats(&String::from_utf8_lossy(&out.stdout))
}

/// Разбирает JSON `xray api statsquery`: суммирует все `...>>>uplink` и `downlink`.
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
