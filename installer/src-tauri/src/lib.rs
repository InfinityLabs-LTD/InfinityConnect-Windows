//! Ядро установщика Infinity Connect.
//!
//! Режимы:
//! - без аргументов — GUI-установка (frameless-окно, 3 экрана);
//! - `--uninstall` — тихое удаление (вызывается из «Программы и компоненты»).

mod install;
mod shortcut;
mod uninstall;

use std::path::PathBuf;

use install::{InstallOptions, DISPLAY_NAME};
use tauri::{AppHandle, Manager};

/// Каталог установки по умолчанию: %ProgramFiles%\Infinity Connect.
#[tauri::command]
fn default_install_dir() -> String {
    program_files().join(DISPLAY_NAME).to_string_lossy().to_string()
}

/// Диалог выбора папки. Возвращает выбранный путь или None.
#[tauri::command]
async fn browse_dir(app: AppHandle, current: String) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let start = PathBuf::from(&current);
    let (tx, rx) = std::sync::mpsc::channel();
    let mut dlg = app.dialog().file();
    if start.is_dir() {
        dlg = dlg.set_directory(start);
    }
    dlg.pick_folder(move |p| {
        let _ = tx.send(p);
    });
    match rx.recv() {
        Ok(Some(p)) => Some(p.to_string()),
        _ => None,
    }
}

/// Запускает установку в фоновом потоке; прогресс идёт событием install://progress.
#[tauri::command]
async fn install(app: AppHandle, opts: InstallOptions) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let res = install::run_install(install::Sink::Gui(&app), &opts);
        let _ = tx.send(res);
    });
    // Ждём завершения install-потока, не блокируя async-runtime событий.
    tokio::task::spawn_blocking(move || rx.recv().unwrap_or(Err("установка прервана".into())))
        .await
        .map_err(|e| e.to_string())?
}

/// Запускает установленное приложение из указанной папки.
///
/// Установщик работает под админом (UAC), но приложение должно стартовать от
/// ОБЫЧНОГО пользователя — иначе VPN-клиент унаследует админ-права. Запуск через
/// `explorer.exe <exe>` сбрасывает elevation (explorer работает от юзера).
#[tauri::command]
fn launch_app(dir: String) -> Result<(), String> {
    let base = if dir.trim().is_empty() {
        program_files().join(DISPLAY_NAME)
    } else {
        PathBuf::from(dir)
    };
    let exe = base.join(install::APP_EXE);
    std::process::Command::new("explorer.exe")
        .arg(&exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Не удалось запустить приложение: {e}"))
}

/// %ProgramFiles% (или дефолт).
fn program_files() -> PathBuf {
    std::env::var("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files"))
}

pub fn run() {
    let args: Vec<String> = std::env::args().collect();

    // Режим удаления — без GUI.
    if args.iter().any(|a| a == "--uninstall") {
        let _ = uninstall::run_uninstall();
        return;
    }

    // Тихая установка из CLI: --install-silent [dir]. Ярлык+автозапуск включены.
    if let Some(pos) = args.iter().position(|a| a == "--install-silent") {
        let dir = args
            .get(pos + 1)
            .cloned()
            .unwrap_or_else(default_install_dir);
        let opts = InstallOptions { dir, desktop_shortcut: true, autostart: true };
        if install::run_install(install::Sink::Stdout, &opts).is_err() {
            std::process::exit(1);
        }
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            default_install_dir,
            browse_dir,
            install,
            launch_app,
        ])
        .setup(|app| {
            // Гарантируем, что окно показано и в фокусе.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ошибка запуска установщика Infinity Connect");
}
