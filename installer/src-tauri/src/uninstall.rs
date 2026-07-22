//! Удаление Infinity Connect: останавливает ядра, удаляет ярлыки, реестр и файлы.
//! Запускается, когда установщик вызван с флагом `--uninstall`.

use std::path::{Path, PathBuf};

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\InfinityConnect";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "InfinityConnect";

use crate::install::DISPLAY_NAME;

/// Полное удаление. Возвращает Ok даже при частичных ошибках очистки, но
/// логирует их — цель максимально почистить систему.
pub fn run_uninstall() -> Result<(), String> {
    let install_dir = read_install_location();
    // Останавливаем ТОЛЬКО ядра из нашей папки установки, чтобы не задеть
    // сторонние приложения с такими же именами процессов (напр. Happ тоже
    // использует sing-box.exe/xray.exe).
    if let Some(ref dir) = install_dir {
        stop_cores_in_dir(dir);
    }
    remove_shortcuts();
    remove_autostart();
    remove_registry();
    if let Some(dir) = install_dir {
        remove_install_dir(&dir);
    }
    Ok(())
}

/// Останавливает процессы, чей исполняемый файл лежит ВНУТРИ `dir` — по PID,
/// а не по имени. Так мы не трогаем одноимённые процессы других приложений.
#[cfg(windows)]
fn stop_cores_in_dir(dir: &Path) {
    let names = ["sing-box.exe", "xray.exe", "hysteria.exe", "infinity-connect.exe"];
    for name in names {
        // WMIC: получаем PID и путь по имени, фильтруем по нашей папке.
        let out = std::process::Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("name='{name}'"),
                "get",
                "ProcessId,ExecutablePath",
                "/format:csv",
            ])
            .output();
        let Ok(out) = out else { continue };
        let text = String::from_utf8_lossy(&out.stdout);
        let dir_lc = dir.to_string_lossy().to_lowercase();
        for line in text.lines() {
            // Строка CSV: Node,ExecutablePath,ProcessId
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 3 {
                continue;
            }
            let path = cols[1].trim();
            let pid = cols[2].trim();
            if path.to_lowercase().starts_with(&dir_lc) && pid.parse::<u32>().is_ok() {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", pid, "/T"])
                    .output();
            }
        }
    }
}

#[cfg(not(windows))]
fn stop_cores_in_dir(_dir: &Path) {}

fn remove_shortcuts() {
    if let Some(programs) = dirs::data_dir().map(|d| d.join(r"Microsoft\Windows\Start Menu\Programs")) {
        let _ = std::fs::remove_file(programs.join(format!("{DISPLAY_NAME}.lnk")));
    }
    if let Some(desktop) = dirs::desktop_dir() {
        let _ = std::fs::remove_file(desktop.join(format!("{DISPLAY_NAME}.lnk")));
    }
}

#[cfg(windows)]
fn read_install_location() -> Option<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(UNINSTALL_KEY).ok()?;
    let loc: String = key.get_value("InstallLocation").ok()?;
    if loc.is_empty() {
        None
    } else {
        Some(PathBuf::from(loc))
    }
}

#[cfg(not(windows))]
fn read_install_location() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn remove_autostart() {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
        let _ = key.delete_value(RUN_VALUE);
    }
}

#[cfg(not(windows))]
fn remove_autostart() {}

#[cfg(windows)]
fn remove_registry() {
    use winreg::enums::*;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let _ = hklm.delete_subkey_all(UNINSTALL_KEY);
}

#[cfg(not(windows))]
fn remove_registry() {}

/// Удаляет каталог установки. Себя (uninstall.exe) удалить сразу нельзя —
/// планируем удаление через отложенный `cmd` после выхода процесса.
fn remove_install_dir(dir: &Path) {
    // Пытаемся удалить всё, кроме работающего uninstall.exe.
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.file_name().map(|n| n == "uninstall.exe").unwrap_or(false) {
                continue;
            }
            if p.is_dir() {
                let _ = std::fs::remove_dir_all(&p);
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
    // Отложенно удаляем сам каталог (с uninstall.exe) после выхода.
    schedule_self_delete(dir);
}

/// Планирует удаление каталога установки после завершения текущего процесса.
/// Наш uninstall.exe держит себя открытым, пока не выйдет, поэтому cmd в цикле
/// ждёт освобождения (несколько попыток), затем удаляет каталог целиком.
fn schedule_self_delete(dir: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let dir_s = dir.to_string_lossy().to_string();
        // Ждём ~2с (даём процессу выйти), затем до 10 попыток rmdir с паузой.
        // Так надёжно удаляется и сам uninstall.exe.
        let cmd = format!(
            "timeout /t 2 /nobreak >nul & \
             for /L %i in (1,1,10) do (rmdir /s /q \"{dir_s}\" 2>nul & \
             if not exist \"{dir_s}\" exit & timeout /t 1 /nobreak >nul)"
        );
        let _ = std::process::Command::new("cmd")
            .args(["/C", &cmd])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::fs::remove_dir_all(dir);
    }
}
