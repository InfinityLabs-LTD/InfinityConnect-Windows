//! Проверка и запрос прав администратора (Windows UAC).
//! wintun-адаптер и правка маршрутов ОС требуют элевации — см. ТЗ, риск №1.

/// Запущен ли процесс с правами администратора.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

/// Перезапускает текущий исполняемый файл с запросом элевации (UAC-диалог).
/// Возвращает true, если элевированный процесс успешно запущен.
#[cfg(windows)]
pub fn relaunch_elevated() -> bool {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let exe = HSTRING::from(exe.as_os_str());
    let verb = HSTRING::from("runas"); // запрос элевации

    unsafe {
        let result = ShellExecuteW(
            None,
            &verb,
            &exe,
            None,
            None,
            SW_SHOWNORMAL,
        );
        // HINSTANCE > 32 → успех (устаревшая, но действующая семантика ShellExecute).
        result.0 as usize > 32
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    true
}

#[cfg(not(windows))]
pub fn relaunch_elevated() -> bool {
    false
}
