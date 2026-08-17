#![cfg(target_os = "windows")]

use std::mem::ManuallyDrop;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::error::Excel2PdfError;

use windows::core::{BSTR, GUID, PCWSTR};
use windows::Win32::Foundation::VARIANT_BOOL;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT,
    DISPPARAMS,
};
use windows::Win32::System::Ole::DISPID_PROPERTYPUT;
use windows::Win32::System::Registry::{RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, KEY_READ};
use windows::Win32::System::Variant::{
    VariantClear, VARIANT, VT_BOOL, VT_BSTR, VT_DISPATCH, VT_EMPTY, VT_I4,
};

const EXCEL_APPLICATION_CLSID: GUID = GUID::from_u128(0x00024500_0000_0000_c000_000000000046);
const XL_TYPE_PDF: i32 = 0;
const XL_QUALITY_STANDARD: i32 = 0;

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

struct ExcelGuard {
    application: Option<IDispatch>,
    workbook: Option<IDispatch>,
}

impl Drop for ExcelGuard {
    fn drop(&mut self) {
        if let Some(workbook) = self.workbook.take() {
            let _ = close_workbook(&workbook);
        }
        if let Some(application) = self.application.take() {
            let _ = quit_application(&application);
        }
    }
}

struct VariantGuard<'a>(&'a mut VARIANT);

impl Drop for VariantGuard<'_> {
    fn drop(&mut self) {
        clear_variant(self.0);
    }
}

fn map_windows_error(err: windows::core::Error) -> Excel2PdfError {
    Excel2PdfError::ConversionFailed(err.to_string())
}

fn variant_path(path: &Path) -> VARIANT {
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let mut variant = VARIANT::default();
    unsafe {
        let inner = &mut *variant.Anonymous.Anonymous;
        inner.vt = VT_BSTR;
        inner.Anonymous.bstrVal = ManuallyDrop::new(BSTR::from_wide(&wide));
    }
    variant
}

fn variant_i4(value: i32) -> VARIANT {
    let mut variant = VARIANT::default();
    unsafe {
        let inner = &mut *variant.Anonymous.Anonymous;
        inner.vt = VT_I4;
        inner.Anonymous.lVal = value;
    }
    variant
}

fn variant_bool(value: bool) -> VARIANT {
    let mut variant = VARIANT::default();
    unsafe {
        let inner = &mut *variant.Anonymous.Anonymous;
        inner.vt = VT_BOOL;
        inner.Anonymous.boolVal = VARIANT_BOOL::from(value);
    }
    variant
}

fn clear_variant(variant: &mut VARIANT) {
    unsafe {
        let _ = VariantClear(variant);
    }
}

fn clear_variants(variants: &mut [VARIANT]) {
    for variant in variants.iter_mut() {
        clear_variant(variant);
    }
}

fn get_dispatch_id(dispatch: &IDispatch, name: &str) -> windows::core::Result<i32> {
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let names = [PCWSTR(wide.as_ptr())];
    let iid_null = GUID::zeroed();
    let mut dispid = 0;
    unsafe {
        dispatch.GetIDsOfNames(
            &iid_null,
            names.as_ptr(),
            names.len() as u32,
            0,
            &mut dispid,
        )?;
    }
    Ok(dispid)
}

fn invoke(
    dispatch: &IDispatch,
    dispid: i32,
    flags: windows::Win32::System::Com::DISPATCH_FLAGS,
    args: &mut [VARIANT],
    named_args: &mut [i32],
) -> windows::core::Result<VARIANT> {
    let params = DISPPARAMS {
        rgvarg: if args.is_empty() {
            std::ptr::null_mut()
        } else {
            args.as_mut_ptr()
        },
        rgdispidNamedArgs: if named_args.is_empty() {
            std::ptr::null_mut()
        } else {
            named_args.as_mut_ptr()
        },
        cArgs: args.len() as u32,
        cNamedArgs: named_args.len() as u32,
    };

    let mut result = VARIANT::default();
    let iid_null = GUID::zeroed();
    unsafe {
        dispatch.Invoke(
            dispid,
            &iid_null,
            0,
            flags,
            &params,
            Some(&mut result),
            None,
            None,
        )?;
    }
    Ok(result)
}

fn invoke_method(
    dispatch: &IDispatch,
    dispid: i32,
    args: &mut [VARIANT],
) -> windows::core::Result<VARIANT> {
    let mut named_args = [];
    invoke(dispatch, dispid, DISPATCH_METHOD, args, &mut named_args)
}

fn invoke_property_get(dispatch: &IDispatch, dispid: i32) -> windows::core::Result<VARIANT> {
    let mut args = [];
    let mut named_args = [];
    invoke(
        dispatch,
        dispid,
        DISPATCH_PROPERTYGET,
        &mut args,
        &mut named_args,
    )
}

fn invoke_property_put(
    dispatch: &IDispatch,
    dispid: i32,
    value: VARIANT,
) -> windows::core::Result<()> {
    let mut args = [value];
    let mut named_args = [DISPID_PROPERTYPUT];
    let result = invoke(
        dispatch,
        dispid,
        DISPATCH_PROPERTYPUT,
        &mut args,
        &mut named_args,
    );
    clear_variants(&mut args);
    let mut result = result?;
    let _result_guard = VariantGuard(&mut result);
    Ok(())
}

fn take_dispatch(variant: &mut VARIANT) -> crate::Result<IDispatch> {
    unsafe {
        let inner = &mut *variant.Anonymous.Anonymous;
        if inner.vt != VT_DISPATCH {
            return Err(Excel2PdfError::ConversionFailed(
                "expected COM dispatch result".into(),
            ));
        }

        let dispatch = ManuallyDrop::into_inner(std::ptr::read(&inner.Anonymous.pdispVal))
            .ok_or_else(|| {
                Excel2PdfError::ConversionFailed("Excel returned a null dispatch pointer".into())
            })?;
        inner.vt = VT_EMPTY;
        Ok(dispatch)
    }
}

fn into_dispatch(mut variant: VARIANT) -> crate::Result<IDispatch> {
    let result = take_dispatch(&mut variant);
    clear_variant(&mut variant);
    result
}

fn close_workbook(workbook: &IDispatch) -> windows::core::Result<()> {
    let dispid = get_dispatch_id(workbook, "Close")?;
    let mut args = [variant_bool(false)];
    let result = invoke_method(workbook, dispid, &mut args);
    clear_variants(&mut args);
    let mut result = result?;
    let _result_guard = VariantGuard(&mut result);
    Ok(())
}

fn quit_application(application: &IDispatch) -> windows::core::Result<()> {
    let dispid = get_dispatch_id(application, "Quit")?;
    let mut args = [];
    let mut result = invoke_method(application, dispid, &mut args)?;
    let _result_guard = VariantGuard(&mut result);
    Ok(())
}

/// Opens a registry subkey for reading. Returns `None` if the key does not exist.
fn open_key(parent: HKEY, subkey: &str) -> Option<HKEY> {
    let wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey = HKEY::default();
    unsafe {
        if RegOpenKeyExW(parent, PCWSTR(wide.as_ptr()), None, KEY_READ, &mut hkey).is_ok() {
            Some(hkey)
        } else {
            None
        }
    }
}

/// Enumerates the direct subkey names of a registry key.
fn enum_subkeys(hkey: HKEY) -> Vec<String> {
    let mut names = Vec::new();
    let mut idx = 0u32;
    loop {
        let mut buf = vec![0u16; 256];
        let mut len = buf.len() as u32;
        let mut class_buf = vec![0u16; 256];
        let mut class_len = class_buf.len() as u32;
        let result = unsafe {
            RegEnumKeyExW(
                hkey,
                idx,
                Some(windows::core::PWSTR(buf.as_mut_ptr())),
                &mut len,
                None,
                Some(windows::core::PWSTR(class_buf.as_mut_ptr())),
                Some(&mut class_len),
                None,
            )
        };
        if result.is_err() {
            break;
        }
        names.push(String::from_utf16_lossy(&buf[..len as usize]));
        idx += 1;
    }
    names
}

/// Search the Windows registry for the LibreOffice installation path.
///
/// Checks `HKLM\SOFTWARE\LibreOffice\LibreOffice` recursively for a `Path`
/// value and returns the path to `soffice.exe`.
pub fn find_libreoffice_in_registry() -> Result<Option<PathBuf>, String> {
    use windows::Win32::System::Registry::{RegQueryValueExW, HKEY_LOCAL_MACHINE, REG_SZ};

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

    fn search(hkey: HKEY) -> Option<String> {
        if let Some(path) = read_path(hkey) {
            return Some(path);
        }
        for sub in enum_subkeys(hkey) {
            if let Some(child) = open_key(hkey, &sub) {
                let found = search(child);
                unsafe {
                    let _ = RegCloseKey(child);
                }
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
        unsafe {
            let _ = RegCloseKey(hkey);
        }
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
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;

    fn has_excel(hkey: HKEY) -> bool {
        let skippable = [
            "ClickToRun",
            "Common",
            "Access",
            "ClickToRunStore",
            "Outlook",
            "PowerPoint",
            "Project",
            "SDXHelper",
            "Visio",
            "Word",
        ];

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
                unsafe {
                    let _ = RegCloseKey(child);
                }
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
    unsafe {
        let _ = RegCloseKey(root);
    }
    Ok(result)
}

/// Converts an Excel file to PDF using Microsoft Excel via COM automation.
pub fn convert_with_excel(excel_path: &Path) -> crate::Result<PathBuf> {
    let excel_path = excel_path.canonicalize()?;
    let pdf_path = excel_path.with_extension("pdf");

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(map_windows_error)?;
    }
    let _com_guard = ComGuard;

    let application: IDispatch =
        unsafe { CoCreateInstance(&EXCEL_APPLICATION_CLSID, None, CLSCTX_LOCAL_SERVER) }
            .map_err(map_windows_error)?;

    let mut excel = ExcelGuard {
        application: Some(application),
        workbook: None,
    };

    let application = excel.application.as_ref().unwrap();

    let display_alerts_id =
        get_dispatch_id(application, "DisplayAlerts").map_err(map_windows_error)?;
    invoke_property_put(application, display_alerts_id, variant_bool(false))
        .map_err(map_windows_error)?;

    let workbooks_id = get_dispatch_id(application, "Workbooks").map_err(map_windows_error)?;
    let workbooks =
        into_dispatch(invoke_property_get(application, workbooks_id).map_err(map_windows_error)?)?;

    let open_id = get_dispatch_id(&workbooks, "Open").map_err(map_windows_error)?;
    let mut open_args = [variant_path(&excel_path)];
    let workbook_variant = invoke_method(&workbooks, open_id, &mut open_args);
    clear_variants(&mut open_args);
    let workbook = into_dispatch(workbook_variant.map_err(map_windows_error)?)?;
    excel.workbook = Some(workbook);

    let workbook = excel.workbook.as_ref().unwrap();
    let export_id = get_dispatch_id(workbook, "ExportAsFixedFormat").map_err(map_windows_error)?;
    let mut export_args = [
        variant_bool(false),
        variant_bool(false),
        variant_bool(true),
        variant_i4(XL_QUALITY_STANDARD),
        variant_path(&pdf_path),
        variant_i4(XL_TYPE_PDF),
    ];
    let export_result = invoke_method(workbook, export_id, &mut export_args);
    clear_variants(&mut export_args);
    let mut export_result = export_result.map_err(map_windows_error)?;
    let _export_result_guard = VariantGuard(&mut export_result);

    close_workbook(workbook).map_err(map_windows_error)?;
    excel.workbook = None;

    quit_application(application).map_err(map_windows_error)?;
    excel.application = None;

    if !pdf_path.exists() {
        return Err(Excel2PdfError::ConversionFailed(
            "PDF was not created".into(),
        ));
    }

    Ok(pdf_path)
}
