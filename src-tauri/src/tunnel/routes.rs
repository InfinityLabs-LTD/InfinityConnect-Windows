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

/// Сбрасывает DNS-кэш ОС (`ipconfig /flushdns`). Без этого после поднятия/снятия
/// туннеля резолверы ОС и приложений держат старые записи → «работает только после
/// перезапуска браузера» и задержки. Вызываем при connect (после маршрутов) и
/// disconnect. Браузерный DoH (Chrome Secure DNS) этим не сбрасывается — там нужен
/// перезапуск, но для системного резолва этого достаточно.
pub fn flush_dns() {
    let mut cmd = Command::new("ipconfig");
    cmd.arg("/flushdns").stdout(Stdio::null()).stderr(Stdio::null());
    no_window(&mut cmd);
    let _ = cmd.status();
}

/// Host-маршрут (`/32`) до IP VPN-сервера через ФИЗИЧЕСКИЙ шлюз — обязателен,
/// иначе `0.0.0.0/1`+`128.0.0.0/1` завернут и пакеты ядра к серверу в tun →
/// петля, соединение не встаёт, «интернета нет». Домен резолвим в IP.
/// Возвращает список добавленных IP (для точного отката).
pub fn add_server_bypass(server_host: &str) -> Vec<String> {
    let ips = resolve_ips(server_host);
    let Some((gw, phys_idx)) = physical_gateway() else {
        return Vec::new(); // не нашли физический шлюз — не рискуем
    };
    let mut added = Vec::new();
    for ip in &ips {
        let _ = run(&[
            "interface", "ipv4", "add", "route",
            &format!("{ip}/32"),
            &format!("interface={phys_idx}"),
            &format!("nexthop={gw}"),
            "metric=1",
            "store=active",
        ]);
        added.push(ip.clone());
    }
    added
}

/// Снимает host-маршруты до сервера (откат при disconnect).
pub fn remove_server_bypass(ips: &[String]) {
    if let Some((_, phys_idx)) = physical_gateway() {
        for ip in ips {
            let _ = run(&[
                "interface", "ipv4", "delete", "route",
                &format!("{ip}/32"),
                &format!("interface={phys_idx}"),
                "store=active",
            ]);
        }
    }
}

/// Резолвит host в IPv4-адреса. Если host уже IP — возвращает его.
fn resolve_ips(host: &str) -> Vec<String> {
    use std::net::ToSocketAddrs;
    if host.parse::<std::net::Ipv4Addr>().is_ok() {
        return vec![host.to_string()];
    }
    // ToSocketAddrs требует порт — 443 как заглушка (используем только IP).
    match (host, 443u16).to_socket_addrs() {
        Ok(addrs) => addrs
            .filter_map(|a| match a.ip() {
                std::net::IpAddr::V4(v4) => Some(v4.to_string()),
                _ => None, // IPv6-серверы MVP не поддерживает в bypass
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Физический шлюз по умолчанию + индекс его интерфейса (НЕ tun-адаптер).
/// Из `show route` берём все `0.0.0.0/0`, отбрасываем строки на tun-интерфейсах
/// (happ/Infinity/sing/Wintun), выбираем реальный шлюз с наименьшей метрикой.
fn physical_gateway() -> Option<(String, u32)> {
    let out = run_capture(&["interface", "ipv4", "show", "route"])?;
    let tun_indices = tun_interface_indices();
    let mut best: Option<(u32, String, u32)> = None; // (metric, gw, idx)
    for line in out.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        // Ищем строку дефолт-маршрута: где-то есть токен "0.0.0.0/0".
        if !toks.iter().any(|t| *t == "0.0.0.0/0") {
            continue;
        }
        // Шлюз — валидный, не-нулевой IPv4-токен (не сам префикс).
        let gw = toks.iter().find(|t| {
            **t != "0.0.0.0/0"
                && t.parse::<std::net::Ipv4Addr>().map(|ip| !ip.is_unspecified()).unwrap_or(false)
        })?;
        // Индекс интерфейса — числовой токен < 1000 (метрика бывает 256, но idx идёт
        // сразу перед именем шлюза; берём последний числовой перед gw).
        let gw_pos = toks.iter().position(|t| t == gw)?;
        let idx = toks[..gw_pos]
            .iter()
            .rev()
            .find_map(|t| t.parse::<u32>().ok().filter(|&n| n < 100000))?;
        // Пропускаем маршруты, висящие на tun-интерфейсах.
        if tun_indices.contains(&idx) {
            continue;
        }
        let metric = toks.iter().filter_map(|t| t.parse::<u32>().ok()).min().unwrap_or(9999);
        let cand = (metric, gw.to_string(), idx);
        if best.as_ref().map(|b| cand.0 < b.0).unwrap_or(true) {
            best = Some(cand);
        }
    }
    best.map(|(_, gw, idx)| (gw, idx))
}

/// Индексы всех tun-подобных адаптеров (их нельзя выбирать физическим шлюзом).
fn tun_interface_indices() -> Vec<u32> {
    let Some(out) = run_capture(&["interface", "ipv4", "show", "interfaces"]) else {
        return Vec::new();
    };
    out.lines()
        .filter(|l| {
            let low = l.to_lowercase();
            low.contains("tun") || low.contains("infinity") || low.contains("happ")
                || low.contains("wintun") || low.contains("wireguard")
        })
        .filter_map(|l| l.split_whitespace().next().and_then(|s| s.parse().ok()))
        .collect()
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
