//! Ядро sing-box как дочерний процесс: поднимает TUN и маршрутизирует трафик по
//! процессам (per-app split-tunnel), проксируя в локальный SOCKS xray-ядра.
//! Статистику трафика в гибриде отдаёт xray (весь трафик идёт через него), поэтому
//! `query_traffic` здесь пустой.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

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
        // Статистику в гибриде отдаёт xray-процесс (весь трафик через него).
        Traffic::default()
    }
}

impl Drop for SingboxProcess {
    fn drop(&mut self) {
        self.stop();
    }
}
