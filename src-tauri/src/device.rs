//! HWID устройства и метаданные для заголовков подписки
//! (аналог Android `DeviceIdProvider.kt`).
//!
//! Панель Remnawave отдаёт реальные конфиги только при запросе подписки с
//! заголовками клиента Happ (User-Agent + x-hwid + x-device-*). Без них —
//! заглушка «Приложение не поддерживается». HWID должен быть стабильным между
//! запусками — на Windows берём `MachineGuid` из реестра (лимит устройств
//! Remnawave считается по нему).

/// Версия клиента в User-Agent (панель отдаёт конфиги для InfinityVPN*; проверено —
/// `InfinityVPN-Windows` панель принимает и возвращает реальный конфиг).
pub const USER_AGENT: &str = "InfinityVPN-Windows/1.0";

/// Стабильный HWID (верхний регистр, как у Happ): `MachineGuid` из реестра.
/// При недоступности реестра — детерминированный fallback от имени ПК.
pub fn hwid() -> String {
    read_machine_guid()
        .unwrap_or_else(|| fallback_id())
        .to_uppercase()
}

/// ОС устройства для заголовка `x-device-os`.
pub fn device_os() -> String {
    "Windows".to_string()
}

/// Версия ОС для `x-ver-os` (например «10.0.26200»).
pub fn os_version() -> String {
    #[cfg(windows)]
    {
        // Читаем из того же ключа CurrentVersion; при промахе — «10».
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(k) = hklm.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion") {
            let major: String = k.get_value("CurrentMajorVersionNumber").map(|v: u32| v.to_string()).unwrap_or_default();
            let build: String = k.get_value("CurrentBuildNumber").unwrap_or_default();
            if !major.is_empty() && !build.is_empty() {
                return format!("{major}.0.{build}");
            }
        }
    }
    "10".to_string()
}

/// Модель устройства для `x-device-model` (имя ПК как приближение).
pub fn device_model() -> String {
    hostname().unwrap_or_else(|| "Windows PC".to_string())
}

#[cfg(windows)]
fn read_machine_guid() -> Option<String> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(r"SOFTWARE\Microsoft\Cryptography").ok()?;
    let guid: String = key.get_value("MachineGuid").ok()?;
    let g = guid.trim();
    if g.is_empty() {
        None
    } else {
        Some(g.to_string())
    }
}

#[cfg(not(windows))]
fn read_machine_guid() -> Option<String> {
    None
}

/// Fallback-идентификатор от имени хоста — детерминированный (без записи на диск).
fn fallback_id() -> String {
    let base = hostname().unwrap_or_else(|| "infinity-connect".to_string());
    // Простой стабильный хэш → UUID-подобная форма.
    let h = fnv1a(base.as_bytes());
    format!("{:016x}{:016x}", h, fnv1a(&h.to_le_bytes()))
}

fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME").ok().filter(|s| !s.is_empty())
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
