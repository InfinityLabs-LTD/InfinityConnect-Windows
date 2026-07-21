//! Шифрование чувствительных данных через Windows DPAPI
//! (CryptProtectData/CryptUnprotectData) — привязка к текущему пользователю ОС.
//! Используется для токенов (аналог Android Keystore).

use crate::error::{AppError, AppResult};

#[cfg(windows)]
pub fn protect(plain: &[u8]) -> AppResult<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: plain.len() as u32,
            pbData: plain.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB::default();

        CryptProtectData(
            &mut in_blob,
            None,
            None,
            None,
            None,
            0,
            &mut out_blob,
        )
        .map_err(|e| AppError::Storage(format!("DPAPI protect: {e}")))?;

        let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
        let result = slice.to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(out_blob.pbData as *mut _));
        Ok(result)
    }
}

#[cfg(windows)]
pub fn unprotect(cipher: &[u8]) -> AppResult<Vec<u8>> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: cipher.len() as u32,
            pbData: cipher.as_ptr() as *mut u8,
        };
        let mut out_blob = CRYPT_INTEGER_BLOB::default();

        CryptUnprotectData(
            &mut in_blob,
            None,
            None,
            None,
            None,
            0,
            &mut out_blob,
        )
        .map_err(|e| AppError::Storage(format!("DPAPI unprotect: {e}")))?;

        let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
        let result = slice.to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(out_blob.pbData as *mut _));
        Ok(result)
    }
}

// Не-Windows заглушки (проект Windows-only, но нужны для проверки на др. хостах).
#[cfg(not(windows))]
pub fn protect(plain: &[u8]) -> AppResult<Vec<u8>> {
    Ok(plain.to_vec())
}

#[cfg(not(windows))]
pub fn unprotect(cipher: &[u8]) -> AppResult<Vec<u8>> {
    Ok(cipher.to_vec())
}
