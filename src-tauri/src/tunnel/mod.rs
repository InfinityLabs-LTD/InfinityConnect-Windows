//! Оркестратор туннеля (замена Android `InfinityVpnService`).
//!
//! Поднимает выбранное ядро (Xray/Hysteria) с готовым конфигом → ждёт появления
//! wintun-адаптера → вешает маршруты ОС → в фоне опрашивает статистику и следит
//! за процессом → при disconnect гасит и откатывает маршруты.
//!
//! Оба ядра сами создают wintun-адаптер (tun-режим), tun2socks не нужен.
//! Требуются права администратора.

mod killswitch;
mod routes;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::engine::selector::{CoreKind, CorePlan};
use crate::error::{AppError, AppResult};
use crate::sidecar::{self, CoreProcess};
use crate::state::{emit_state, emit_stats, TunnelState};

/// Шлюз tun-сети Xray (ядро слушает .2/30, шлюз — .1). Для Hysteria маршруты в
/// его собственном конфиге (`tun.route`), доп. netsh не нужен.
const XRAY_TUN_GATEWAY: &str = "10.10.0.1";
const STATS_INTERVAL: Duration = Duration::from_secs(1);
const TUN_WAIT_ATTEMPTS: u32 = 40;
const TUN_WAIT_STEP: Duration = Duration::from_millis(250);
/// Каждые N тиков (по STATS_INTERVAL) проверяем целостность маршрутов (handover).
const ROUTE_CHECK_TICKS: u32 = 3;

/// Менеджер туннеля: одно активное подключение за раз.
#[derive(Clone)]
pub struct TunnelManager {
    exe_dir: PathBuf,
    inner: Arc<Mutex<Option<Active>>>,
}

struct Active {
    process: Arc<Mutex<Box<dyn CoreProcess>>>,
    monitor: JoinHandle<()>,
    if_index: Option<u32>,
    /// Включён ли kill-switch (чтобы снять при disconnect).
    kill_switch: bool,
}

impl TunnelManager {
    pub fn new(exe_dir: PathBuf) -> Self {
        Self { exe_dir, inner: Arc::new(Mutex::new(None)) }
    }

    pub async fn is_connected(&self) -> bool {
        self.inner.lock().await.is_some()
    }

    /// Поднимает туннель по плану ядра. `kill_switch` — блокировать не-VPN трафик.
    pub async fn connect(&self, app: AppHandle, plan: CorePlan, kill_switch: bool) -> AppResult<()> {
        self.disconnect(&app).await;
        emit_state(&app, TunnelState::Connecting);

        // 1. Запуск ядра (оно само создаёт wintun-адаптер).
        let process = sidecar::start(plan.kind, &self.exe_dir, &plan.config_json, plan.stats_port)?;
        let process = Arc::new(Mutex::new(process));

        // 2. Ждём появления адаптера.
        let if_index = wait_for_tun(plan.tun_name).await;
        if if_index.is_none() {
            let mut p = process.lock().await;
            let hint = p.exit_status().map(|c| format!(" (ядро вышло с кодом {c})")).unwrap_or_default();
            p.stop();
            emit_state(&app, TunnelState::Error(format!(
                "не удалось создать TUN-адаптер{hint}. Запустите приложение от имени администратора."
            )));
            return Err(AppError::Other("TUN-адаптер не создан".into()));
        }

        // 3. Маршруты ОС + DNS нужны только Xray (Hysteria задаёт их в конфиге).
        if plan.kind == CoreKind::Xray {
            if let Some(idx) = if_index {
                routes::add_default_routes(idx, XRAY_TUN_GATEWAY)?;
                // DNS на tun-адаптер — анти-утечка (системный резолвер в туннель).
                routes::set_dns(idx, &["1.1.1.1", "8.8.8.8"]);
            }
        }

        // 4. Kill-switch: блок не-VPN трафика (разрешаем сервер + локалку). Ставим
        //    ДО объявления Connected, чтобы при мгновенном обрыве не было утечки.
        if kill_switch {
            killswitch::enable(&plan.server_ip);
        }

        // 5. Фоновый монитор: статистика + процесс + переустановка маршрутов
        //    (network handover при смене сети Wi-Fi↔ethernet).
        let monitor = spawn_monitor(app.clone(), process.clone(), self.inner.clone(), if_index, plan.kind);

        *self.inner.lock().await = Some(Active { process, monitor, if_index, kill_switch });
        emit_state(&app, TunnelState::Connected(plan.remark));
        Ok(())
    }

    /// Гасит активный туннель, откатывает маршруты и снимает kill-switch.
    pub async fn disconnect(&self, app: &AppHandle) {
        let active = self.inner.lock().await.take();
        if let Some(active) = active {
            active.monitor.abort();
            if let Some(idx) = active.if_index {
                routes::remove_default_routes(idx);
            }
            active.process.lock().await.stop();
            // Kill-switch снимаем только при ЯВНОМ отключении (при обрыве ядра он
            // остаётся активным — в этом его смысл: не пускать трафик мимо VPN).
            if active.kill_switch {
                killswitch::disable();
            }
        }
        emit_state(app, TunnelState::Disconnected);
    }
}

/// Ждёт, пока ОС покажет wintun-адаптер с нужным именем. `None` — не появился.
async fn wait_for_tun(name: &str) -> Option<u32> {
    for _ in 0..TUN_WAIT_ATTEMPTS {
        if let Some(idx) = routes::tun_interface_index(name) {
            return Some(idx);
        }
        tokio::time::sleep(TUN_WAIT_STEP).await;
    }
    None
}

/// Фоновая задача: раз в секунду читает статистику и эмитит; если ядро упало —
/// эмитит ошибку и снимает активное состояние.
fn spawn_monitor(
    app: AppHandle,
    process: Arc<Mutex<Box<dyn CoreProcess>>>,
    slot: Arc<Mutex<Option<Active>>>,
    if_index: Option<u32>,
    kind: CoreKind,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut prev_up = 0u64;
        let mut prev_down = 0u64;
        let mut tick: u32 = 0;
        loop {
            tokio::time::sleep(STATS_INTERVAL).await;

            let mut p = process.lock().await;
            if let Some(code) = p.exit_status() {
                drop(p);
                emit_state(&app, TunnelState::Error(format!("ядро остановилось (код {code})")));
                *slot.lock().await = None;
                break;
            }
            let t = p.query_traffic();
            drop(p);

            let up_speed = t.uplink.saturating_sub(prev_up);
            let down_speed = t.downlink.saturating_sub(prev_down);
            prev_up = t.uplink;
            prev_down = t.downlink;
            emit_stats(&app, t.uplink, t.downlink, up_speed, down_speed);

            // Network handover: раз в ~3с проверяем, что наши default-маршруты на
            // tun живы (смена сети Wi-Fi↔ethernet сбрасывает таблицу маршрутов).
            tick = tick.wrapping_add(1);
            if kind == CoreKind::Xray && tick % ROUTE_CHECK_TICKS == 0 {
                if let Some(idx) = if_index {
                    if !routes::default_routes_present(idx) {
                        let _ = routes::add_default_routes(idx, XRAY_TUN_GATEWAY);
                        routes::set_dns(idx, &["1.1.1.1", "8.8.8.8"]);
                    }
                }
            }
        }
    })
}
