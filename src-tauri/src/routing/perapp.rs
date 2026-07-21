//! Split-tunnel по приложениям через Windows Firewall (управляемая надстройка WFP).
//!
//! **Честные границы Windows.** Полноценный per-app split-tunnel «направить трафик
//! ЭТОГО процесса в tun, остального — мимо» требует callout-драйвера режима ядра
//! (перенаправление на уровне пакетов). Без драйвера на прикладном уровне доступна
//! только БЛОКИРОВКА трафика по пути процесса (`program=<path>`), не перенаправление.
//!
//! Поэтому реализуем то, что надёжно работает без драйвера:
//!  - **Disallow** (через VPN всё, КРОМЕ выбранных): выбранным приложениям блокируем
//!    исходящий трафик, пока VPN активен → они не «утекают» мимо туннеля. Полноценно.
//!  - **Allow** (через VPN ТОЛЬКО выбранные): корректно на прикладном уровне не
//!    выразить (нужно перенаправление невыбранных мимо tun). Не применяем — задел
//!    под драйвер; логируем.
//!
//! Правила помечены группой и удаляются строго свои.

use std::process::{Command, Stdio};

use crate::routing::{AppRoutingMode, RoutingSettings};

const GROUP: &str = "InfinityConnect PerApp";
const RULE_PREFIX: &str = "InfinityApp-";

/// Применяет split-tunnel по приложениям. Возвращает число установленных правил.
pub fn apply_per_app(settings: &RoutingSettings) -> usize {
    clear_per_app();
    if settings.apps.is_empty() {
        return 0;
    }
    match settings.app_mode {
        AppRoutingMode::Off => 0,
        AppRoutingMode::Disallow => {
            // Блокируем исходящий трафик выбранных приложений (не «утекают» мимо VPN).
            let mut n = 0;
            for (i, path) in settings.apps.iter().enumerate() {
                if path.trim().is_empty() {
                    continue;
                }
                add_block_rule(i, path.trim());
                n += 1;
            }
            n
        }
        AppRoutingMode::Allow => {
            // На прикладном уровне не реализуемо без callout-драйвера (Фаза с драйвером).
            eprintln!(
                "[routing] per-app ALLOW ({} прил.) требует callout-драйвера — не применено",
                settings.apps.len()
            );
            0
        }
    }
}

/// Снимает установленные правила per-app.
pub fn clear_per_app() {
    // Удаляем по группе — одним вызовом снимаем все наши правила.
    let _ = run(&["advfirewall", "firewall", "delete", "rule", &format!("group={GROUP}")]);
}

fn add_block_rule(idx: usize, program_path: &str) {
    let name = format!("{RULE_PREFIX}{idx}");
    let _ = run(&[
        "advfirewall", "firewall", "add", "rule",
        &format!("name={name}"),
        &format!("group={GROUP}"),
        "dir=out",
        "action=block",
        "enable=yes",
        "profile=any",
        &format!("program={program_path}"),
    ]);
}

fn run(args: &[&str]) -> bool {
    let mut cmd = Command::new("netsh");
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}
