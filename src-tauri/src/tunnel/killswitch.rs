//! Kill-switch: блокировка не-VPN трафика при обрыве ядра (аналог флагов
//! VpnService на Android). Реализован через Windows Firewall (advfirewall) —
//! управляемая надстройка над WFP, обратимая и не требующая callout-драйвера.
//!
//! Модель: при активации создаём блокирующие правила на весь исходящий трафик,
//! КРОМЕ: трафика через tun-адаптер (сам VPN) и до IP VPN-сервера (иначе ядро
//! не сможет держать соединение). При деактивации — удаляем свои правила.
//!
//! Правила помечаются группой INFINITY_GROUP — чистим строго свои.

use std::process::{Command, Stdio};

const GROUP: &str = "InfinityConnect KillSwitch";
const RULE_BLOCK: &str = "InfinityKS-Block";
const RULE_ALLOW_SERVER: &str = "InfinityKS-AllowServer";
const RULE_ALLOW_LOOPBACK: &str = "InfinityKS-AllowLoopback";

/// Включает kill-switch: блок исходящего, кроме сервера и локалхоста.
/// `server_ip` — адрес VPN-сервера (чтобы ядро держало соединение мимо блока).
/// tun-трафик к серверу и так идёт через физический интерфейс (direct-outbound).
pub fn enable(server_ip: &str) {
    // На всякий случай снимаем прежние правила (перезапуск).
    disable();

    // 1) Разрешить трафик до VPN-сервера (иначе ядро не переподключится).
    if !server_ip.is_empty() && server_ip != "—" {
        netsh_add_rule(&[
            RULE_ALLOW_SERVER, "dir=out", "action=allow",
            &format!("remoteip={server_ip}"),
        ]);
    }
    // 2) Разрешить локальную сеть/loopback (DNS-локалхост, LAN).
    netsh_add_rule(&[
        RULE_ALLOW_LOOPBACK, "dir=out", "action=allow",
        "remoteip=127.0.0.0/8,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,169.254.0.0/16",
    ]);
    // 3) Блокировать всё остальное исходящее.
    netsh_add_rule(&[RULE_BLOCK, "dir=out", "action=block", "remoteip=any"]);
}

/// Выключает kill-switch: удаляет все наши правила.
pub fn disable() {
    for name in [RULE_BLOCK, RULE_ALLOW_SERVER, RULE_ALLOW_LOOPBACK] {
        let _ = run(&["advfirewall", "firewall", "delete", "rule", &format!("name={name}")]);
    }
}

fn netsh_add_rule(extra: &[&str]) {
    let name = extra[0];
    let mut args = vec![
        "advfirewall".to_string(),
        "firewall".to_string(),
        "add".to_string(),
        "rule".to_string(),
        format!("name={name}"),
        format!("group={GROUP}"),
        "enable=yes".to_string(),
        "profile=any".to_string(),
    ];
    // Остальные параметры (dir/action/remoteip) — после имени.
    for a in &extra[1..] {
        args.push(a.to_string());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let _ = run(&refs);
}

fn run(args: &[&str]) {
    let mut cmd = Command::new("netsh");
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let _ = cmd.status();
}
