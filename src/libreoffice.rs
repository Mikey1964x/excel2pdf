use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Excel2PdfError;

/// Converts an Excel file to PDF using LibreOffice in headless mode.
///
/// The generated PDF is written to the same directory as the source file.
/// Only the first page is retained in the output PDF.
pub fn convert_with_libreoffice(excel_path: &Path) -> crate::Result<PathBuf> {
    let lo_path = find_libreoffice_bin()?;

    let excel_path = excel_path.canonicalize().map_err(|e| {
        Excel2PdfError::ConversionFailed(format!("failed to get absolute path: {}", e))
    })?;

    let out_dir = excel_path
        .parent()
        .ok_or_else(|| Excel2PdfError::ConversionFailed("no parent directory".into()))?;

    // Each concurrent LibreOffice process needs its own user-profile directory;
    // without this, multiple headless instances collide on the same lock file.
    let profile_dir = tempfile::tempdir().map_err(|e| {
        Excel2PdfError::ConversionFailed(format!("failed to create temp dir: {}", e))
    })?;

    let profile_url = dir_to_file_url(profile_dir.path());

    let status = Command::new(&lo_path)
        .args([
            "--headless",
            "--norestore",
            &format!("--env:UserInstallation={}", profile_url),
            "--convert-to",
            "pdf",
            "--outdir",
        ])
        .arg(out_dir)
        .arg(&excel_path)
        .status()
        .map_err(|e| {
            Excel2PdfError::ConversionFailed(format!("failed to run LibreOffice: {}", e))
        })?;

    if !status.success() {
        return Err(Excel2PdfError::ConversionFailed(format!(
            "LibreOffice exited with status {}",
            status
        )));
    }

    // Construct the expected PDF path (same stem, .pdf extension).
    let stem = excel_path
        .file_stem()
        .ok_or_else(|| Excel2PdfError::ConversionFailed("file has no stem".into()))?;

    let pdf_path = out_dir.join(stem).with_extension("pdf");

    // Trim to first page only.
    crate::pdf::remove_all_but_first_page(&pdf_path)?;

    Ok(pdf_path)
}

/// Convert a directory path to a `file://` URL suitable for LibreOffice's
/// `--env:UserInstallation` argument.
fn dir_to_file_url(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if !s.starts_with('/') {
        // Windows: `C:/...` → `/C:/...`
        s = format!("/{}", s);
    }
    format!("file://{}", s)
}

// ---------------------------------------------------------------------------
// Platform-specific resolution of the LibreOffice binary path
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn find_libreoffice_bin() -> crate::Result<PathBuf> {
    use std::env;

    if let Ok(val) = env::var("LIBREOFFICE_PATH") {
        return Ok(PathBuf::from(val));
    }

    // Try `which libreoffice`, then `which libreoffice24.8`.
    for name in &["libreoffice", "libreoffice24.8"] {
        if let Some(p) = which_cmd(name) {
            return Ok(p);
        }
    }

    Err(Excel2PdfError::LibreOfficeNotInstalled)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn which_cmd(name: &str) -> Option<PathBuf> {
    let out = Command::new("which").arg(name).output().ok()?;
    if out.status.success() {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn find_libreoffice_bin() -> crate::Result<PathBuf> {
    use std::env;
    use crate::windows::find_libreoffice_in_registry;

    if let Ok(val) = env::var("LIBREOFFICE_PATH") {
        return Ok(PathBuf::from(val));
    }

    find_libreoffice_in_registry().map_err(|e| {
        Excel2PdfError::ConversionFailed(format!("registry error: {}", e))
    })?.ok_or(Excel2PdfError::LibreOfficeNotInstalled)
}
