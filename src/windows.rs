#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};

use crate::error::Excel2PdfError;

/// Search the Windows registry for the LibreOffice installation path.
///
/// Checks `HKLM\SOFTWARE\LibreOffice\LibreOffice` recursively for a `Path`
/// value and returns the path to `soffice.exe`.
pub fn find_libreoffice_in_registry() -> Result<Option<PathBuf>, String> {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY,
        HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };
    use windows::core::PCWSTR;

    fn open_key(parent: HKEY, subkey: &str) -> Option<HKEY> {
        let wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();
        unsafe {
            RegOpenKeyExW(parent, PCWSTR(wide.as_ptr()), 0, KEY_READ, &mut hkey).ok()?;
        }
        Some(hkey)
    }

    fn read_path(hkey: HKEY) -> Option<String> {
        let name: Vec<u16> = "Path\0".encode_utf16().collect();
        let mut buf = vec![0u16; 512];
        let mut size = (buf.len() * 2) as u32;
        let mut kind = 0u32;
        let result = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind as *mut u32 as *mut _),
                Some(buf.as_mut_ptr() as *mut u8),
                Some(&mut size),
            )
        };
        if result.is_ok() && kind == REG_SZ.0 {
            let len = size as usize / 2;
            let s: String = String::from_utf16_lossy(&buf[..len.saturating_sub(1)]);
            Some(s)
        } else {
            None
        }
    }

    fn enum_subkeys(hkey: HKEY) -> Vec<String> {
        let mut names = Vec::new();
        let mut idx = 0u32;
        loop {
            let mut buf = vec![0u16; 256];
            let mut len = buf.len() as u32;
            let result = unsafe {
                RegEnumKeyExW(hkey, idx, windows::core::PWSTR(buf.as_mut_ptr()), &mut len, None, windows::core::PWSTR::null(), None, None)
            };
            if result.is_err() {
                break;
            }
            let name = String::from_utf16_lossy(&buf[..len as usize]);
            names.push(name);
            idx += 1;
        }
        names
    }

    fn search(hkey: HKEY) -> Option<String> {
        if let Some(path) = read_path(hkey) {
            return Some(path);
        }
        for sub in enum_subkeys(hkey) {
            if let Some(child) = open_key(hkey, &sub) {
                let found = search(child);
                unsafe { let _ = RegCloseKey(child); }
                if found.is_some() {
                    return found;
                }
            }
        }
        None
    }

    let root = open_key(HKEY_LOCAL_MACHINE, r"SOFTWARE\LibreOffice\LibreOffice");
    let result = root.and_then(|hkey| {
        let found = search(hkey);
        unsafe { let _ = RegCloseKey(hkey); }
        found
    });

    match result {
        Some(dir) => {
            let exe = PathBuf::from(dir).join("soffice.exe");
            if exe.exists() {
                Ok(Some(exe))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Reports whether Microsoft Excel is installed on this machine by checking the registry.
pub fn is_excel_installed() -> Result<bool, String> {
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY,
        HKEY_LOCAL_MACHINE, KEY_READ,
    };
    use windows::core::PCWSTR;

    fn open_key(parent: HKEY, subkey: &str) -> Option<HKEY> {
        let wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hkey = HKEY::default();
        unsafe {
            RegOpenKeyExW(parent, PCWSTR(wide.as_ptr()), 0, KEY_READ, &mut hkey).ok()?;
        }
        Some(hkey)
    }

    fn enum_subkeys(hkey: HKEY) -> Vec<String> {
        let mut names = Vec::new();
        let mut idx = 0u32;
        loop {
            let mut buf = vec![0u16; 256];
            let mut len = buf.len() as u32;
            let result = unsafe {
                RegEnumKeyExW(hkey, idx, windows::core::PWSTR(buf.as_mut_ptr()), &mut len, None, windows::core::PWSTR::null(), None, None)
            };
            if result.is_err() {
                break;
            }
            names.push(String::from_utf16_lossy(&buf[..len as usize]));
            idx += 1;
        }
        names
    }

    fn has_excel(hkey: HKEY) -> bool {
        let skippable = ["ClickToRun", "Common", "Access",
            "ClickToRunStore", "Outlook", "PowerPoint",
            "Project", "SDXHelper", "Visio", "Word"];

        for sub in enum_subkeys(hkey) {
            if sub == "Excel" {
                return true;
            }
            if skippable.contains(&sub.as_str()) {
                continue;
            }
            // Recurse into unknown subkeys
            if let Some(child) = open_key(hkey, &sub) {
                let found = has_excel(child);
                unsafe { let _ = RegCloseKey(child); }
                if found {
                    return true;
                }
            }
        }
        false
    }

    let root = match open_key(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Office") {
        Some(k) => k,
        None => return Ok(false),
    };
    let result = has_excel(root);
    unsafe { let _ = RegCloseKey(root); }
    Ok(result)
}

/// Converts an Excel file to PDF using Microsoft Excel via COM automation.
pub fn convert_with_excel(_excel_path: &Path) -> crate::Result<PathBuf> {
    // Excel COM automation is not yet fully implemented in the Rust port.
    // Install LibreOffice for Excel-to-PDF conversion on Windows.
    Err(Excel2PdfError::ConversionFailed(
        "Excel COM automation is not yet implemented; \
         please install LibreOffice for Excel-to-PDF conversion on Windows"
            .into(),
    ))
}
