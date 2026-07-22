//! Перечисление установленных приложений для выбора в split-tunnel.
//!
//! Источник — ярлыки Start Menu (`.lnk`): это «настоящие» пользовательские
//! приложения (в отличие от сотен служебных exe в Program Files). Возвращаем имя
//! exe (для sing-box `process_name`) + отображаемое имя (из имени ярлыка).
//! Discord-подобные приложения несут вспомогательные exe (Update.exe) в своей
//! папке — их добавляем к записи, чтобы split-tunnel ловил и обновлятор.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Одно установленное приложение: отображаемое имя + все имена exe (основной +
/// вспомогательные из той же папки, напр. Discord.exe + Update.exe).
#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    /// Отображаемое имя (из ярлыка), напр. «Discord».
    pub name: String,
    /// Имена exe для process_name (основной + соседние), напр. ["Discord.exe","Update.exe"].
    pub exe_names: Vec<String>,
}

/// Сканирует Start Menu (общий + пользовательский) и возвращает приложения,
/// отсортированные по имени, без дублей по основному exe.
pub fn list_installed() -> Vec<InstalledApp> {
    let mut by_exe: BTreeMap<String, InstalledApp> = BTreeMap::new();

    for dir in start_menu_dirs() {
        collect_lnks(&dir, &mut by_exe);
    }

    let mut apps: Vec<InstalledApp> = by_exe.into_values().collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// Каталоги Start Menu: ProgramData (общий) + AppData (пользовательский).
fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(pd) = std::env::var("ProgramData") {
        dirs.push(PathBuf::from(pd).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(ad) = std::env::var("AppData") {
        dirs.push(PathBuf::from(ad).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    dirs
}

/// Рекурсивно обходит каталог, резолвит `.lnk` → target exe, добавляет в карту.
fn collect_lnks(dir: &Path, out: &mut BTreeMap<String, InstalledApp>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lnks(&path, out);
        } else if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("lnk")).unwrap_or(false) {
            if let Some(target) = resolve_lnk_target(&path) {
                add_app(&path, &target, out);
            }
        }
    }
}

/// Добавляет приложение в карту, собирая релевантные exe из папки приложения.
/// Особый случай: у Discord/Slack/… ярлык указывает на `Update.exe` в корне, а
/// основной exe — в подпапке `app-<version>\`. Поэтому если цель — апдейтер,
/// ищем «настоящий» exe по имени приложения в подпапках.
fn add_app(lnk: &Path, target_exe: &Path, out: &mut BTreeMap<String, InstalledApp>) {
    let Some(exe_name) = target_exe.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let low = exe_name.to_lowercase();
    if low.contains("uninstall") || low.contains("unins000") {
        return;
    }

    // Отображаемое имя — из имени ярлыка (без .lnk).
    let display = lnk
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(exe_name)
        .to_string();
    // Имя приложения без пробелов — эвристика для основного exe (Discord → discord.exe).
    let app_slug = display.replace(' ', "").to_lowercase();

    let mut exe_names: Vec<String> = Vec::new();
    let mut push = |n: &str, list: &mut Vec<String>| {
        if list.len() < 12 && !list.iter().any(|e: &String| e.eq_ignore_ascii_case(n)) {
            list.push(n.to_string());
        }
    };
    // Всегда добавляем то, на что указывает ярлык (Update.exe для Discord).
    push(exe_name, &mut exe_names);

    // Ищем в папке приложения основной exe + соседей. Папка = родитель цели.
    if let Some(parent) = target_exe.parent() {
        collect_relevant_exes(parent, &app_slug, 0, &mut exe_names);
    }

    // Ключ карты — по отображаемому имени (не по exe, т.к. Update.exe у многих одинаков).
    let key = app_slug.clone();
    if out.contains_key(&key) {
        return;
    }
    out.insert(key, InstalledApp { name: display, exe_names });
}

/// Рекурсивно (до 2 уровней) собирает из папки приложения: exe, совпадающий с
/// именем приложения (Discord.exe), и известные вспомогательные (Update.exe).
fn collect_relevant_exes(dir: &Path, app_slug: &str, depth: u8, out: &mut Vec<String>) {
    if depth > 2 || out.len() >= 12 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            // Спускаемся только в осмысленные подпапки (app-<ver>, current, bin).
            if dname.starts_with("app-") || dname == "current" || dname == "bin" || dname.starts_with("app") {
                collect_relevant_exes(&path, app_slug, depth + 1, out);
            }
        } else if let Some(n) = path.file_name().and_then(|n| n.to_str()) {
            let nl = n.to_lowercase();
            if !nl.ends_with(".exe") || nl.contains("uninstall") {
                continue;
            }
            let stem = nl.trim_end_matches(".exe");
            // Основной exe приложения (Discord.exe) или обновлятор.
            if stem == app_slug || nl == "update.exe" || app_slug.contains(stem) || stem.contains(app_slug) {
                if out.len() < 12 && !out.iter().any(|e| e.eq_ignore_ascii_case(n)) {
                    out.push(n.to_string());
                }
            }
        }
    }
}

/// Читает target из `.lnk` без внешних зависимостей: парсит бинарный формат
/// Shell Link (MS-SHLLINK) — вытаскивает строку пути к цели из LinkInfo/StringData.
/// Достаточно найти в файле путь, оканчивающийся на `.exe`.
fn resolve_lnk_target(lnk: &Path) -> Option<PathBuf> {
    let bytes = std::fs::read(lnk).ok()?;
    // Ищем ASCII/UTF-16 подстроку с "…\<name>.exe". Простой, но надёжный для нашей
    // задачи подход: сканируем печатаемые ASCII-последовательности.
    let mut best: Option<String> = None;
    let mut cur = String::new();
    let flush = |cur: &mut String, best: &mut Option<String>| {
        if cur.to_lowercase().ends_with(".exe") && cur.contains('\\') {
            // Предпочитаем более длинный (полный путь).
            if best.as_ref().map(|b| cur.len() > b.len()).unwrap_or(true) {
                *best = Some(cur.clone());
            }
        }
        cur.clear();
    };
    // ASCII-проход.
    for &b in &bytes {
        if (0x20..0x7f).contains(&b) {
            cur.push(b as char);
        } else {
            flush(&mut cur, &mut best);
        }
    }
    flush(&mut cur, &mut best);

    // UTF-16LE проход (пути в StringData обычно Unicode).
    let mut cur16 = String::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let ch = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if (0x20..0x7f).contains(&ch) {
            cur16.push(ch as u8 as char);
        } else {
            flush(&mut cur16, &mut best);
        }
        i += 2;
    }
    flush(&mut cur16, &mut best);

    best.map(PathBuf::from).filter(|p| p.file_name().is_some())
}
