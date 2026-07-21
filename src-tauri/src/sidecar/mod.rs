//! Запуск и менеджмент ядер (`xray.exe` / `hysteria.exe`) как дочерних процессов.
//! Оба ядра сами поднимают wintun-адаптер (tun-режим), поэтому tun2socks не нужен.
//! Требуются права администратора (создание адаптера + маршруты).

mod hysteria;
mod xray;

pub use hysteria::HysteriaProcess;
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

/// Запускает ядро нужного вида по готовому конфигу. `stats_port` — порт API
/// статистики (у каждого ядра свой протокол её чтения).
pub fn start(kind: CoreKind, exe_dir: &Path, config_json: &str, stats_port: u16) -> AppResult<Box<dyn CoreProcess>> {
    match kind {
        CoreKind::Xray => Ok(Box::new(XrayProcess::start(exe_dir, config_json, stats_port)?)),
        CoreKind::Hysteria => Ok(Box::new(HysteriaProcess::start(exe_dir, config_json, stats_port)?)),
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
