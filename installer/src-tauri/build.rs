use std::path::PathBuf;

fn main() {
    // Готовим payload.zip в OUT_DIR для include_bytes! (single-file установщик).
    // Источник — installer/payload.zip (генерит build-payload.ps1 перед сборкой).
    // Если архива нет (dev/CI без payload) — кладём пустой файл: код это поймёт и
    // будет искать распакованную папку payload/ рядом с exe, как раньше.
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("payload.zip");
    // installer/payload.zip — на два уровня выше src-tauri.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("payload.zip");
    if src.is_file() {
        std::fs::copy(&src, &out).expect("копирование payload.zip в OUT_DIR");
        println!("cargo:rerun-if-changed={}", src.display());
    } else if !out.exists() {
        std::fs::write(&out, []).expect("пустой payload.zip в OUT_DIR");
    }

    // Встраиваем манифест с requireAdministrator (установка в Program Files/HKLM).
    let manifest = include_str!("installer.manifest");
    let attrs = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(manifest));
    tauri_build::try_build(attrs).expect("ошибка сборки установщика");
}
