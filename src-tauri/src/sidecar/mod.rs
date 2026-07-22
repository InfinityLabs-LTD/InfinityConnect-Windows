//! Запуск и менеджмент ядер (`xray.exe` / `hysteria.exe`) как дочерних процессов.
//! Оба ядра сами поднимают wintun-адаптер (tun-режим), поэтому tun2socks не нужен.
//! Требуются права администратора (создание адаптера + маршруты).

mod hysteria;
mod singbox;
mod xray;

pub use hysteria::HysteriaProcess;
pub use singbox::SingboxProcess;
pub use xray::XrayProcess;

use std::path::Path;

use crate::engine::selector::CoreKind;
use crate::error::AppResult;

/// Скрывает окно консоли дочернего процесса на Windows.
#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Счётчики трафика (байты) с момента старта ядра.
#[derive(Debug, Clone, Copy, Default)]
pub struct Traffic {
    pub uplink: u64,
    pub downlink: u64,
}

/// Общий интерфейс управляемого ядра для оркестратора туннеля.
pub trait CoreProcess: Send {
    /// `Some(code)`, если процесс уже завершился.
    fn exit_status(&mut self) -> Option<i32>;
    /// Останавливает ядро и чистит временные файлы.
    fn stop(&mut self);
    /// Суммарная статистика трафика.
    fn query_traffic(&self) -> Traffic;
}

/// Запускает ядро-прокси (xray/hysteria) в socks-режиме по готовому конфигу.
/// `stats_port` — порт API статистики (у каждого ядра свой протокол её чтения).
pub fn start_proxy(kind: CoreKind, exe_dir: &Path, config_json: &str, stats_port: u16) -> AppResult<Box<dyn CoreProcess>> {
    match kind {
        CoreKind::Xray => Ok(Box::new(XrayProcess::start(exe_dir, config_json, stats_port)?)),
        CoreKind::Hysteria => Ok(Box::new(HysteriaProcess::start(exe_dir, config_json, stats_port)?)),
    }
}

/// Запускает sing-box (TUN + routing по процессам).
pub fn start_singbox(exe_dir: &Path, config_json: &str) -> AppResult<Box<dyn CoreProcess>> {
    Ok(Box::new(SingboxProcess::start(exe_dir, config_json)?))
}

/// stderr ядра → `<exe_dir>/<name>_stderr.log` (truncate при каждом старте).
/// При ошибке создания файла — молча `null` (диагностика не критична для работы).
pub(crate) fn core_log(exe_dir: &Path, name: &str) -> std::process::Stdio {
    let path = exe_dir.join(format!("{name}_stderr.log"));
    match std::fs::File::create(&path) {
        Ok(f) => std::process::Stdio::from(f),
        Err(_) => std::process::Stdio::null(),
    }
}

/// Применяет флаг «скрыть окно» к команде (Windows).
#[cfg(windows)]
pub(crate) fn hide_window(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn hide_window(_cmd: &mut std::process::Command) {}
