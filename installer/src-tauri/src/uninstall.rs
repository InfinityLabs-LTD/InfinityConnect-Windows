//! Удаление Infinity Connect: останавливает ядра, удаляет ярлыки, реестр и файлы.
//! Запускается, когда установщик вызван с флагом `--uninstall`.

use std::path::{Path, PathBuf};

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\InfinityConnect";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "InfinityConnect";

use crate::install::DISPLAY_NAME;

/// Полное удаление. Возвращает Ok даже при частичных ошибках очистки.
///
/// Чтобы надёжно удалить и саму папку установки (включая работающий
/// uninstall.exe), деинсталлятор сначала копирует себя в %TEMP% и
/// перезапускается оттуда — уже temp-копия сносит всю папню целиком.
pub fn run_uninstall() -> Result<(), String> {
    if !running_from_temp() {
        // Первый запуск (из папки установки): реле в temp и выходим.
        relaunch_from_temp();
        return Ok(());
    }

    // Мы уже в temp-копии — можно безопасно удалять всё.
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
        // Ждём, пока папка освободится, и удаляем целиком (uninstall.exe там
        // больше не запущен — мы работаем из temp).
        remove_dir_with_retries(&dir);
    }
    Ok(())
}

/// Признак: запущены ли мы из %TEMP% (реле-копия).
fn running_from_temp() -> bool {
    match std::env::current_exe() {
        Ok(exe) => {
            let tmp = std::env::temp_dir();
            exe.starts_with(&tmp)
        }
        Err(_) => false,
    }
}

/// Копирует себя в %TEMP% и запускает оттуда с --uninstall, затем текущий
/// процесс завершается (освобождая папку установки).
fn relaunch_from_temp() {
    let Ok(cur) = std::env::current_exe() else { return };
    let tmp = std::env::temp_dir().join("infinity-uninstall.exe");
    if std::fs::copy(&cur, &tmp).is_err() {
        // Не смогли скопировать — удаляем как есть (папка может частично остаться).
        let install_dir = read_install_location();
        stop_shortcuts_registry(&install_dir);
        if let Some(dir) = install_dir {
            remove_dir_with_retries(&dir);
        }
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new(&tmp)
            .arg("--uninstall")
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new(&tmp).arg("--uninstall").spawn();
    }
}

/// Общая очистка ярлыков/реестра/автозапуска (используется в аварийной ветке).
fn stop_shortcuts_registry(install_dir: &Option<PathBuf>) {
    if let Some(ref dir) = install_dir {
        stop_cores_in_dir(dir);
    }
    remove_shortcuts();
    remove_autostart();
    remove_registry();
}

/// Удаляет каталог целиком с несколькими попытками (файлы могут освобождаться
/// не мгновенно после остановки процессов).
fn remove_dir_with_retries(dir: &Path) {
    for _ in 0..15 {
        if !dir.exists() {
            return;
        }
        let _ = std::fs::remove_dir_all(dir);
        if !dir.exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}

/// Останавливает ВСЕ процессы, чей исполняемый файл лежит ВНУТРИ `dir` — по пути,
/// а не по имени (чтобы не задеть одноимённые процессы других приложений, напр.
/// Happ). Использует PowerShell CIM (надёжнее устаревшего wmic) и ждёт, пока
/// процессы реально завершатся, иначе папка не удалится.
pub(crate) fn stop_cores_in_dir_pub(dir: &Path) {
    stop_cores_in_dir(dir);
}

#[cfg(windows)]
fn stop_cores_in_dir(dir: &Path) {
    use std::os::windows::process::CommandExt;
    let dir_s = dir.to_string_lossy().replace('\'', "''");
    // Убиваем все процессы, чей ExecutablePath начинается с папки установки,
    // затем ждём их завершения (до ~6с).
    let ps = format!(
        "$d='{dir_s}'; \
         for ($i=0; $i -lt 15; $i++) {{ \
           $p = Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -and $_.ExecutablePath.StartsWith($d, [System.StringComparison]::OrdinalIgnoreCase) }}; \
           if (-not $p) {{ break }}; \
           $p | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}; \
           Start-Sleep -Milliseconds 400 \
         }}"
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output();
}

#[cfg(not(windows))]
fn stop_cores_in_dir(_dir: &Path) {}

fn remove_shortcuts() {
    // Пути ДОЛЖНЫ совпадать с install.rs: общесистемное меню Пуск (%ProgramData%)
    // и общий рабочий стол (%PUBLIC%), а не профиль текущего (админ) пользователя.
    let program_data = std::env::var("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData"));
    let start_menu = program_data
        .join(r"Microsoft\Windows\Start Menu\Programs")
        .join(format!("{DISPLAY_NAME}.lnk"));
    let _ = std::fs::remove_file(&start_menu);

    let public = std::env::var("PUBLIC")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Public"));
    let desktop = public.join("Desktop").join(format!("{DISPLAY_NAME}.lnk"));
    let _ = std::fs::remove_file(&desktop);
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

