//! Создание Windows-ярлыков (.lnk) через COM-интерфейс IShellLink + IPersistFile.

#[cfg(windows)]
pub fn create(target: &std::path::Path, lnk_path: &std::path::Path) -> Result<(), String> {
    use windows::core::{Interface, GUID, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    // COM apartment для этого вызова.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let result = (|| -> Result<(), String> {
        unsafe {
            let shell_link: IShellLinkW =
                CoCreateInstance(&ShellLink as *const GUID as *const _, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("CoCreateInstance: {e}"))?;

            let target_w = to_wide(&target.to_string_lossy());
            shell_link
                .SetPath(PCWSTR(target_w.as_ptr()))
                .map_err(|e| format!("SetPath: {e}"))?;

            // Рабочая папка = папка приложения.
            if let Some(dir) = target.parent() {
                let dir_w = to_wide(&dir.to_string_lossy());
                let _ = shell_link.SetWorkingDirectory(PCWSTR(dir_w.as_ptr()));
            }
            // Иконка — из самого exe.
            let _ = shell_link.SetIconLocation(PCWSTR(target_w.as_ptr()), 0);

            let persist: IPersistFile = shell_link
                .cast()
                .map_err(|e| format!("cast IPersistFile: {e}"))?;
            let lnk_w = to_wide(&lnk_path.to_string_lossy());
            persist
                .Save(PCWSTR(lnk_w.as_ptr()), true)
                .map_err(|e| format!("Save lnk: {e}"))?;
        }
        Ok(())
    })();

    unsafe {
        CoUninitialize();
    }
    result
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(not(windows))]
pub fn create(_target: &std::path::Path, _lnk_path: &std::path::Path) -> Result<(), String> {
    Ok(())
}
