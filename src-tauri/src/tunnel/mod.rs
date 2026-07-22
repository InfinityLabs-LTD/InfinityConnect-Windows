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

use crate::engine::selector::HybridPlan;
use crate::error::{AppError, AppResult};
use crate::sidecar::{self, CoreProcess};
use crate::state::{emit_state, emit_stats, TunnelState};

const STATS_INTERVAL: Duration = Duration::from_secs(1);
const TUN_WAIT_ATTEMPTS: u32 = 40;
const TUN_WAIT_STEP: Duration = Duration::from_millis(250);

/// Менеджер туннеля: одно активное подключение за раз.
#[derive(Clone)]
pub struct TunnelManager {
    exe_dir: PathBuf,
    inner: Arc<Mutex<Option<Active>>>,
}

struct Active {
    /// Ядро-прокси (xray/hysteria) — socks.
    proxy: Arc<Mutex<Box<dyn CoreProcess>>>,
    /// sing-box — TUN + routing.
    singbox: Arc<Mutex<Box<dyn CoreProcess>>>,
    monitor: JoinHandle<()>,
    /// Включён ли kill-switch (чтобы снять при disconnect).
    kill_switch: bool,
    /// Host-маршруты до сервера (обход туннеля) — для отката при disconnect.
    server_bypass_ips: Vec<String>,
}

impl TunnelManager {
    pub fn new(exe_dir: PathBuf) -> Self {
        Self { exe_dir, inner: Arc::new(Mutex::new(None)) }
    }

    pub async fn is_connected(&self) -> bool {
        self.inner.lock().await.is_some()
    }

    /// Поднимает гибридный туннель: ядро-прокси (socks) + sing-box (TUN+routing).
    /// `kill_switch` — блокировать не-VPN трафик.
    pub async fn connect(&self, app: AppHandle, plan: HybridPlan, kill_switch: bool) -> AppResult<()> {
        self.disconnect(&app).await;
        emit_state(&app, TunnelState::Connecting);

        // 1. Ядро-прокси (xray/hysteria) — слушает локальный SOCKS, TUN НЕ поднимает.
        let proxy = sidecar::start_proxy(
            plan.proxy_kind,
            &self.exe_dir,
            &plan.proxy_config_json,
            plan.stats_port,
        )?;
        let proxy = Arc::new(Mutex::new(proxy));

        // 2. Host-маршрут до сервера в обход туннеля — ДО подъёма TUN, иначе трафик
        //    ядра-прокси к серверу уйдёт в tun (петля) и соединение не встанет.
        let server_bypass_ips = routes::add_server_bypass(&plan.server_ip);

        // 3. Ждём, пока socks ядра-прокси поднимется (иначе sing-box гонит в никуда).
        if !wait_for_port(plan.proxy_socks_port).await {
            let mut p = proxy.lock().await;
            let hint = p.exit_status().map(|c| format!(" (ядро-прокси код {c})")).unwrap_or_default();
            p.stop();
            routes::remove_server_bypass(&server_bypass_ips);
            emit_state(&app, TunnelState::Error(format!("прокси-ядро не поднялось{hint}")));
            return Err(AppError::Other("прокси-ядро не поднялось".into()));
        }

        // 4. sing-box: поднимает TUN и маршрутизирует по процессам → в socks прокси.
        let singbox = sidecar::start_singbox(&self.exe_dir, &plan.singbox_config_json)?;
        let singbox = Arc::new(Mutex::new(singbox));

        // 5. Ждём появления TUN-адаптера sing-box.
        let if_index = wait_for_tun(plan.tun_name).await;
        if if_index.is_none() {
            let mut sb = singbox.lock().await;
            let hint = sb.exit_status().map(|c| format!(" (sing-box код {c})")).unwrap_or_default();
            sb.stop();
            proxy.lock().await.stop();
            routes::remove_server_bypass(&server_bypass_ips);
            emit_state(&app, TunnelState::Error(format!(
                "не удалось создать TUN-адаптер{hint}. Запустите приложение от имени администратора."
            )));
            return Err(AppError::Other("TUN-адаптер не создан".into()));
        }

        // Сброс DNS-кэша ОС: иначе старые записи резолвятся мимо туннеля и трафик
        // «оживает» только после ручного перезапуска браузера / с задержкой.
        routes::flush_dns();

        // 6. Kill-switch: блок не-VPN трафика. Ставим ДО объявления Connected.
        if kill_switch {
            killswitch::enable(&plan.server_ip);
        }

        // 7. Монитор следит за ОБОИМИ процессами; статистику берёт с ядра-прокси.
        let monitor = spawn_monitor(app.clone(), proxy.clone(), singbox.clone(), self.inner.clone());

        *self.inner.lock().await = Some(Active {
            proxy,
            singbox,
            monitor,
            kill_switch,
            server_bypass_ips,
        });
        emit_state(&app, TunnelState::Connected(plan.remark));
        Ok(())
    }

    /// Гасит активный туннель, откатывает маршруты и снимает kill-switch.
    pub async fn disconnect(&self, app: &AppHandle) {
        let active = self.inner.lock().await.take();
        if let Some(active) = active {
            active.monitor.abort();
            // sing-box гасим ПЕРВЫМ: он снимает auto_route/TUN и возвращает маршруты.
            active.singbox.lock().await.stop();
            // Потом ядро-прокси.
            active.proxy.lock().await.stop();
            routes::remove_server_bypass(&active.server_bypass_ips);
            // Сброс DNS-кэша: убрать записи, зарезолвленные через туннельный DNS.
            routes::flush_dns();
            // Kill-switch снимаем только при ЯВНОМ отключении (при обрыве ядра он
            // остаётся активным — в этом его смысл: не пускать трафик мимо VPN).
            if active.kill_switch {
                killswitch::disable();
            }
        }
        emit_state(app, TunnelState::Disconnected);
    }
}

/// Ждёт, пока локальный порт начнёт слушать (ядро-прокси подняло socks).
async fn wait_for_port(port: u16) -> bool {
    for _ in 0..TUN_WAIT_ATTEMPTS {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        tokio::time::sleep(TUN_WAIT_STEP).await;
    }
    false
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

/// Фоновая задача: читает статистику с ядра-прокси и эмитит; если ЛЮБОЙ из двух
/// процессов (прокси или sing-box) упал — эмитит ошибку и снимает состояние.
/// Маршруты/handover ведёт сам sing-box (`auto_route`), доп. netsh не нужен.
fn spawn_monitor(
    app: AppHandle,
    proxy: Arc<Mutex<Box<dyn CoreProcess>>>,
    singbox: Arc<Mutex<Box<dyn CoreProcess>>>,
    slot: Arc<Mutex<Option<Active>>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut prev_up = 0u64;
        let mut prev_down = 0u64;
        loop {
            tokio::time::sleep(STATS_INTERVAL).await;

            // Проверяем оба процесса.
            if let Some(code) = proxy.lock().await.exit_status() {
                emit_state(&app, TunnelState::Error(format!("прокси-ядро остановилось (код {code})")));
                *slot.lock().await = None;
                break;
            }
            if let Some(code) = singbox.lock().await.exit_status() {
                emit_state(&app, TunnelState::Error(format!("sing-box остановился (код {code})")));
                *slot.lock().await = None;
                break;
            }

            let t = proxy.lock().await.query_traffic();
            let up_speed = t.uplink.saturating_sub(prev_up);
            let down_speed = t.downlink.saturating_sub(prev_down);
            prev_up = t.uplink;
            prev_down = t.downlink;
            emit_stats(&app, t.uplink, t.downlink, up_speed, down_speed);
        }
    })
}
