//! Настройка маршрутов ОС на tun-адаптер (аналог Android VpnService.Builder
//! addRoute, но здесь — команды маршрутизации Windows).
//!
//! MVP: заворачиваем весь трафик в tun через две half-default маршрута
//! (0.0.0.0/1 + 128.0.0.0/1) — они перекрывают дефолт, не удаляя его (проще
//! откатить). Трафик до самого VPN-сервера идёт мимо (иначе петля) — это
//! обеспечивает direct-outbound ядра + сохранённый физический маршрут.
//!
//! IP Helper API вместо netsh — Фаза 7 (полировка). Для MVP — `netsh`/`route`.

use std::process::{Command, Stdio};

use crate::error::AppResult;

/// Индекс интерфейса wintun-адаптера по его имени (для команд маршрутизации).
pub fn tun_interface_index(name: &str) -> Option<u32> {
    // `netsh int ipv4 show interfaces` → ищем строку с именем адаптера.
    let out = run_capture(&["interface", "ipv4", "show", "interfaces"])?;
    for line in out.lines() {
        if line.contains(name) {
            // Первый столбец — Idx.
            if let Some(idx) = line.split_whitespace().next().and_then(|s| s.parse().ok()) {
                return Some(idx);
            }
        }
    }
    None
}

/// Навешивает default-маршруты (0/1 + 128/1) на tun-адаптер.
/// gateway — адрес шлюза tun (10.10.0.1, ядро слушает .2/30).
pub fn add_default_routes(if_index: u32, gateway: &str) -> AppResult<()> {
    for prefix in ["0.0.0.0/1", "128.0.0.0/1"] {
        let _ = run(&[
            "interface", "ipv4", "add", "route",
            prefix,
            &format!("interface={if_index}"),
            &format!("nexthop={gateway}"),
            "metric=1",
            "store=active",
        ]);
    }
    Ok(())
}

/// Снимает добавленные маршруты (откат при disconnect).
pub fn remove_default_routes(if_index: u32) {
    for prefix in ["0.0.0.0/1", "128.0.0.0/1"] {
        let _ = run(&[
            "interface", "ipv4", "delete", "route",
            prefix,
            &format!("interface={if_index}"),
            "store=active",
        ]);
    }
}

/// Активны ли наши default-маршруты на tun-интерфейсе. Смена сети (Wi-Fi↔ethernet)
/// может сбросить их — тогда оркестратор переустанавливает (network handover).
pub fn default_routes_present(if_index: u32) -> bool {
    let Some(out) = run_capture(&["interface", "ipv4", "show", "route"]) else {
        return true; // не смогли проверить — не трогаем
    };
    // Ищем обе half-default записи на нашем интерфейсе.
    let idx = if_index.to_string();
    let has = |prefix: &str| {
        out.lines().any(|l| l.contains(prefix) && l.split_whitespace().any(|t| t == idx))
    };
    has("0.0.0.0/1") && has("128.0.0.0/1")
}

/// Прописывает DNS на tun-интерфейс (анти-утечка: системный DNS в туннель).
/// Ядро резолвит через свои dns-серверы, но системный резолвер должен идти в tun.
pub fn set_dns(if_index: u32, servers: &[&str]) {
    // Первый — основной, остальные — дополнительные.
    if let Some((first, rest)) = servers.split_first() {
        let _ = run(&[
            "interface", "ipv4", "set", "dnsservers",
            &format!("name={if_index}"),
            "static", first, "primary",
        ]);
        for (i, dns) in rest.iter().enumerate() {
            let _ = run(&[
                "interface", "ipv4", "add", "dnsservers",
                &format!("name={if_index}"),
                dns,
                &format!("index={}", i + 2),
            ]);
        }
    }
}

// ── netsh-обёртки ──

fn run(args: &[&str]) -> AppResult<()> {
    let mut cmd = Command::new("netsh");
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());
    no_window(&mut cmd);
    let _ = cmd.status();
    Ok(())
}

fn run_capture(args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("netsh");
    cmd.args(args).stderr(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
}
