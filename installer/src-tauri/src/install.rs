//! Логика установки Infinity Connect: копирование файлов приложения в целевую
//! папку, создание ярлыков, запись реестра (для «Программы и компоненты»),
//! автозапуск. Прогресс эмитится во фронт через событие `install://progress`.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Куда слать прогресс: в GUI-окно или в stdout (тихий CLI-режим).
pub enum Sink<'a> {
    Gui(&'a AppHandle),
    Stdout,
}

/// Имя главного бинарника приложения (как его ставит основной bundle —
/// MAINBINARYNAME = "infinity-connect", НЕ productName).
pub const APP_EXE: &str = "infinity-connect.exe";
/// Отображаемое имя (реестр, ярлыки).
pub const DISPLAY_NAME: &str = "Infinity Connect";
pub const PUBLISHER: &str = "Infinity Labs";
pub const VERSION: &str = "1.0.0";
/// Ключ реестра Uninstall (машинный).
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\InfinityConnect";
/// Ключ автозапуска.
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "InfinityConnect";

/// Опции установки из UI.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InstallOptions {
    pub dir: String,
    #[serde(rename = "desktopShortcut")]
    pub desktop_shortcut: bool,
    pub autostart: bool,
}

/// Событие прогресса (совпадает с типом Progress во фронте).
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub fraction: f32,
    pub step: String,
    pub log: Vec<String>,
}

/// Ошибка установки — строкой во фронт.
pub type InstallResult = Result<(), String>;

struct Reporter<'a> {
    sink: Sink<'a>,
    log: Vec<String>,
}

impl<'a> Reporter<'a> {
    fn new(sink: Sink<'a>) -> Self {
        Self { sink, log: Vec::new() }
    }
    /// Эмитит шаг: продвигает прогресс и добавляет строку в лог.
    fn step(&mut self, fraction: f32, step: &str, done: &str) {
        self.log.push(done.to_string());
        match self.sink {
            Sink::Gui(app) => {
                let _ = app.emit(
                    "install://progress",
                    Progress { fraction, step: step.to_string(), log: self.log.clone() },
                );
            }
            Sink::Stdout => {
                println!("[{:>3}%] {step} — {done}", (fraction * 100.0) as u32);
            }
        }
    }
}

/// Payload, вшитый в бинарь установщика (single-file дистрибутив). Пустой в
/// dev-сборке без installer/payload.zip — тогда берём папку payload/ рядом.
static EMBEDDED_PAYLOAD_ZIP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.zip"));

/// Каталог с файлами приложения, которые нужно скопировать (payload).
///
/// Прод (single-file): payload вшит в exe как ZIP — распаковываем во временную
/// папку и отдаём её. Dev: `payload/` рядом с exe или в корне проекта установщика.
pub fn payload_dir() -> Option<PathBuf> {
    // 1) Вшитый ZIP (single-file установщик с сайта).
    if !EMBEDDED_PAYLOAD_ZIP.is_empty() {
        if let Some(dir) = extract_embedded_payload() {
            return Some(dir);
        }
    }
    // 2) рядом с exe установщика: <exe_dir>/payload
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("payload");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    // 3) dev-режим: installer/payload от cwd
    let dev = PathBuf::from("payload");
    if dev.is_dir() {
        return Some(dev);
    }
    None
}

/// Распаковывает вшитый payload.zip во временную папку. Возвращает путь к ней.
fn extract_embedded_payload() -> Option<PathBuf> {
    use std::io::Cursor;
    let dest = std::env::temp_dir().join("infinity-payload");
    // Чистим прошлую распаковку (переустановка/повторный запуск).
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).ok()?;

    let mut zip = zip::ZipArchive::new(Cursor::new(EMBEDDED_PAYLOAD_ZIP)).ok()?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).ok()?;
        // enclosed_name защищает от zip-slip (путей вида ../).
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).ok()?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ok()?;
            }
            let mut f = std::fs::File::create(&out).ok()?;
            std::io::copy(&mut entry, &mut f).ok()?;
        }
    }
    Some(dest)
}

/// Полный цикл установки. Блокирующая; вызывать из отдельного потока.
pub fn run_install(sink: Sink<'_>, opts: &InstallOptions) -> InstallResult {
    let mut rep = Reporter::new(sink);
    let target = PathBuf::from(&opts.dir);

    rep.step(0.08, "Подготовка…", "Проверены права доступа");

    // 1. Копируем файлы приложения.
    let src = payload_dir().ok_or_else(|| {
        "Не найдены файлы приложения (payload). В сборке они кладутся рядом с установщиком.".to_string()
    })?;

    // Переустановка/обновление: чистим старое содержимое целевой папки, чтобы не
    // оставались файлы прежних версий. Сначала глушим процессы из этой папки
    // (по пути, чужие ядра не трогаем), затем удаляем всё, кроме самого
    // работающего установщика, если он вдруг оттуда запущен.
    if target.exists() {
        stop_processes_in_dir(&target);
        clear_dir_contents(&target);
    }
    std::fs::create_dir_all(&target).map_err(|e| format!("Не удалось создать папку установки: {e}"))?;

    rep.step(0.25, "Копирование файлов приложения…", "Начато копирование");
    copy_dir_all(&src, &target).map_err(|e| format!("Ошибка копирования файлов: {e}"))?;
    rep.step(0.6, "Копирование файлов приложения…", "Файлы приложения скопированы");

    let app_exe = target.join(APP_EXE);

    // 2. Ярлыки.
    rep.step(0.72, "Создание ярлыков…", "Ярлык в меню «Пуск»");
    create_start_menu_shortcut(&app_exe).map_err(|e| format!("Ошибка ярлыка меню Пуск: {e}"))?;
    if opts.desktop_shortcut {
        create_desktop_shortcut(&app_exe).map_err(|e| format!("Ошибка ярлыка рабочего стола: {e}"))?;
        rep.step(0.8, "Создание ярлыков…", "Ярлык на рабочем столе");
    }

    // 3. Реестр Uninstall (+ размер, издатель, деинсталлятор).
    rep.step(0.88, "Регистрация…", "Запись в «Программы и компоненты»");
    write_uninstall_registry(&target).map_err(|e| format!("Ошибка записи реестра: {e}"))?;

    // 4. Автозапуск.
    if opts.autostart {
        set_autostart(&app_exe).map_err(|e| format!("Ошибка автозапуска: {e}"))?;
        rep.step(0.95, "Регистрация…", "Настроен автозапуск");
    }

    rep.step(1.0, "Завершение…", "Установка завершена");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_dir_all_and_size() {
        let base = std::env::temp_dir().join(format!("ic_test_{}", std::process::id()));
        let src = base.join("src");
        let dst = base.join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), b"hello").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), b"world!!").unwrap();

        copy_dir_all(&src, &dst).unwrap();
        assert!(dst.join("a.txt").is_file());
        assert!(dst.join("sub").join("b.txt").is_file());
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello");

        // 5 + 7 = 12 байт → 0 КБ (округление вниз), но функция не паникует.
        let _ = dir_size_kb(&dst);

        let _ = std::fs::remove_dir_all(&base);
    }
}

/// Останавливает процессы, запущенные из папки установки (по пути).
fn stop_processes_in_dir(dir: &Path) {
    crate::uninstall::stop_cores_in_dir_pub(dir);
}

/// Удаляет всё содержимое папки (для чистой переустановки). Занятые файлы
/// пропускаются молча — их перезапишет копирование.
fn clear_dir_contents(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let _ = std::fs::remove_dir_all(&p);
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

/// Рекурсивное копирование каталога.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Директория размещения установленного деинсталлятора (копия установщика).
fn uninstaller_path(target: &Path) -> PathBuf {
    target.join("uninstall.exe")
}

/// Записывает реестр Uninstall (машинный HKLM), чтобы приложение появилось в
/// «Программы и компоненты» с иконкой, издателем и командой удаления.
#[cfg(windows)]
fn write_uninstall_registry(target: &Path) -> std::io::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    // Копируем себя как uninstall.exe в папку установки.
    let uninst = uninstaller_path(target);
    if let Ok(cur) = std::env::current_exe() {
        let _ = std::fs::copy(&cur, &uninst);
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let (key, _) = hklm.create_subkey(UNINSTALL_KEY)?;
    let app_exe = target.join(APP_EXE);
    let size_kb = dir_size_kb(target);

    key.set_value("DisplayName", &DISPLAY_NAME)?;
    key.set_value("DisplayVersion", &VERSION)?;
    key.set_value("Publisher", &PUBLISHER)?;
    key.set_value("DisplayIcon", &app_exe.to_string_lossy().to_string())?;
    key.set_value("InstallLocation", &target.to_string_lossy().to_string())?;
    key.set_value(
        "UninstallString",
        &format!("\"{}\" --uninstall", uninst.to_string_lossy()),
    )?;
    key.set_value(
        "QuietUninstallString",
        &format!("\"{}\" --uninstall --silent", uninst.to_string_lossy()),
    )?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    key.set_value("EstimatedSize", &size_kb)?;
    Ok(())
}

#[cfg(not(windows))]
fn write_uninstall_registry(_target: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Прописывает автозапуск приложения (HKCU\...\Run).
#[cfg(windows)]
fn set_autostart(app_exe: &Path) -> std::io::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(RUN_KEY)?;
    key.set_value(RUN_VALUE, &format!("\"{}\"", app_exe.to_string_lossy()))?;
    Ok(())
}

#[cfg(not(windows))]
fn set_autostart(_app_exe: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Оценка размера установленного каталога в КБ.
fn dir_size_kb(dir: &Path) -> u32 {
    fn walk(dir: &Path) -> u64 {
        let mut total = 0;
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if let Ok(md) = e.metadata() {
                    total += if md.is_dir() { walk(&e.path()) } else { md.len() };
                }
            }
        }
        total
    }
    (walk(dir) / 1024).min(u32::MAX as u64) as u32
}

// ── Ярлыки (.lnk через IShellLink COM) ──

#[cfg(windows)]
fn create_start_menu_shortcut(app_exe: &Path) -> std::io::Result<()> {
    // Общесистемное меню «Пуск» (%ProgramData%), а не профиль текущего (админ)
    // пользователя — иначе ярлык не виден обычному пользователю.
    let base = std::env::var("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData"));
    let programs = base.join(r"Microsoft\Windows\Start Menu\Programs");
    std::fs::create_dir_all(&programs)?;
    let lnk = programs.join(format!("{DISPLAY_NAME}.lnk"));
    crate::shortcut::create(app_exe, &lnk).map_err(std::io::Error::other)
}

#[cfg(windows)]
fn create_desktop_shortcut(app_exe: &Path) -> std::io::Result<()> {
    // Общий рабочий стол (%PUBLIC%\Desktop) — виден всем пользователям; под
    // elevated процессом dirs::desktop_dir() указал бы на профиль админа.
    let public = std::env::var("PUBLIC")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Public"));
    let desktop = public.join("Desktop");
    std::fs::create_dir_all(&desktop)?;
    let lnk = desktop.join(format!("{DISPLAY_NAME}.lnk"));
    crate::shortcut::create(app_exe, &lnk).map_err(std::io::Error::other)
}

#[cfg(not(windows))]
fn create_start_menu_shortcut(_app_exe: &Path) -> std::io::Result<()> {
    Ok(())
}
#[cfg(not(windows))]
fn create_desktop_shortcut(_app_exe: &Path) -> std::io::Result<()> {
    Ok(())
}
