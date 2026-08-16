//! # excel2pdf
//!
//! Converts Excel (`.xlsx`) files to PDF and merges PDF files.
//!
//! Supports Windows, Linux, and macOS. On Windows, LibreOffice is preferred
//! if installed, with fallback to Microsoft Excel. On Linux and macOS,
//! LibreOffice must be installed.
//!
//! ## Concurrency
//!
//! Up to [`DEFAULT_MAX_CONCURRENCY`] Excel-to-PDF conversions may run
//! concurrently. Callers that exceed this limit will block until a slot is
//! free. Call [`set_max_concurrency`] once at startup to adjust the limit.
//!
//! PDF merge operations ([`combine_pdfs`]) are exclusive: only one may run
//! at a time. If another merge is already in progress,
//! [`Excel2PdfError::AlreadyProcessing`] is returned immediately.
//!
//! ## Example
//!
//! ```no_run
//! excel2pdf::set_max_concurrency(4);
//!
//! let pdf = excel2pdf::convert_excel_to_pdf("report.xlsx").unwrap();
//! println!("PDF written to {}", pdf.display());
//! ```

pub mod error;
pub mod libreoffice;
pub mod pdf;

#[cfg(target_os = "windows")]
pub(crate) mod windows;

pub use error::Excel2PdfError;

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, TryLockError};

/// Result type alias for this crate.
pub type Result<T> = std::result::Result<T, Excel2PdfError>;

/// Default maximum number of concurrent Excel-to-PDF conversions.
pub const DEFAULT_MAX_CONCURRENCY: usize = 3;

// ---------------------------------------------------------------------------
// Counting semaphore (Mutex<usize> + Condvar)
// ---------------------------------------------------------------------------

static SEM_CAPACITY: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_CONCURRENCY);
static SEM: Lazy<(Mutex<usize>, Condvar)> = Lazy::new(|| (Mutex::new(0), Condvar::new()));

fn acquire_slot() -> SlotGuard<'static> {
    let guard = SEM.0.lock().unwrap();
    let guard = SEM
        .1
        .wait_while(guard, |count| {
            if *count < SEM_CAPACITY.load(Ordering::Relaxed) {
                *count += 1;
                false // stop waiting — we have a slot
            } else {
                true // keep waiting
            }
        })
        .unwrap();
    SlotGuard(guard)
}

struct SlotGuard<'a>(std::sync::MutexGuard<'a, usize>);
impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        *self.0 -= 1;
        SEM.1.notify_one();
    }
}

// ---------------------------------------------------------------------------
// Combine-PDFs mutex
// ---------------------------------------------------------------------------

static COMBINE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Sets the maximum number of concurrent [`convert_excel_to_pdf`] calls.
///
/// Must be called **before** any conversions begin. Panics if `n < 1`.
///
/// # Panics
///
/// Panics when `n < 1`.
pub fn set_max_concurrency(n: usize) {
    assert!(n >= 1, "excel2pdf: set_max_concurrency: n must be >= 1");
    SEM_CAPACITY.store(n, Ordering::Relaxed);
}

/// Converts an Excel file to PDF and returns the path of the generated PDF.
///
/// On Windows, LibreOffice is used when available; otherwise Microsoft Excel
/// is used. On Linux and macOS, LibreOffice must be installed.
///
/// Up to the limit set by [`set_max_concurrency`] conversions may run in
/// parallel. Excess callers block until a slot becomes available.
pub fn convert_excel_to_pdf<P: AsRef<Path>>(excel_file: P) -> Result<PathBuf> {
    let _slot = acquire_slot();
    convert_excel_to_pdf_impl(excel_file.as_ref())
}

/// Merges one or more PDF files into a single PDF written to `output_pdf_file`.
///
/// Only one merge may run at a time. If another call is already in progress,
/// [`Excel2PdfError::AlreadyProcessing`] is returned immediately.
pub fn combine_pdfs<P: AsRef<Path>>(pdf_files: &[P], output_pdf_file: &Path) -> Result<PathBuf> {
    match COMBINE_LOCK.try_lock() {
        Ok(_guard) => {
            pdf::merge_pdfs(pdf_files, output_pdf_file)?;
            Ok(output_pdf_file.to_path_buf())
        }
        Err(TryLockError::WouldBlock) => Err(Excel2PdfError::AlreadyProcessing),
        Err(TryLockError::Poisoned(e)) => {
            // Recover from a poisoned lock; another thread panicked but the
            // data itself is fine.
            let _guard = e.into_inner();
            pdf::merge_pdfs(pdf_files, output_pdf_file)?;
            Ok(output_pdf_file.to_path_buf())
        }
    }
}

// ---------------------------------------------------------------------------
// Platform-specific conversion
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn convert_excel_to_pdf_impl(excel_file: &Path) -> Result<PathBuf> {
    libreoffice::convert_with_libreoffice(excel_file)
}

#[cfg(target_os = "windows")]
fn convert_excel_to_pdf_impl(excel_file: &Path) -> Result<PathBuf> {
    // Prefer LibreOffice when installed.
    match libreoffice::find_libreoffice_bin() {
        Ok(_) => return libreoffice::convert_with_libreoffice(excel_file),
        Err(Excel2PdfError::LibreOfficeNotInstalled) => {}
        Err(e) => return Err(e),
    }

    // Fall back to Microsoft Excel COM automation.
    if windows::is_excel_installed().unwrap_or(false) {
        return windows::convert_with_excel(excel_file);
    }

    Err(Excel2PdfError::ConverterNotFound)
}
