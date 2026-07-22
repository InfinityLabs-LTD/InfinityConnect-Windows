fn main() {
    // Встраиваем манифест с requireAdministrator (установка в Program Files/HKLM).
    let manifest = include_str!("installer.manifest");
    let attrs = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(manifest));
    tauri_build::try_build(attrs).expect("ошибка сборки установщика");
}
