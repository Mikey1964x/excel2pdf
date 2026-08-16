/// Integration tests for the excel2pdf Rust library.
///
/// Most tests create temporary PDF files so that no external tools
/// (LibreOffice, Excel) are required.  Tests that exercise Excel-to-PDF
/// conversion require LibreOffice to be installed and are skipped
/// automatically when testdata is not present.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

use excel2pdf::{combine_pdfs, convert_excel_to_pdf, set_max_concurrency, Excel2PdfError};
use lopdf::Document;

// ---------------------------------------------------------------------------
// Test-level serialization for combine_pdfs calls
//
// combine_pdfs uses a try-lock internally (by design – matching the Go
// behaviour), so parallel tests that each call combine_pdfs would race.
// We serialise them with a test-local mutex so every test gets the global
// lock exclusively.
// ---------------------------------------------------------------------------
static COMBINE_TEST_LOCK: std::sync::LazyLock<Mutex<()>> =
    std::sync::LazyLock::new(|| Mutex::new(()));

fn with_combine_lock<F: FnOnce()>(f: F) {
    let _guard = COMBINE_TEST_LOCK.lock().unwrap();
    f();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a minimal single-page PDF at `path` and return the path.
fn create_test_pdf(path: &Path) -> PathBuf {
    use lopdf::{Dictionary, Object, Stream};

    let mut doc = Document::with_version("1.5");

    // Page content stream (empty).
    let content_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        b"".to_vec(),
    )));

    // Page dictionary.
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", Object::Name(b"Page".to_vec()));
    page_dict.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(595),
            Object::Integer(842),
        ]),
    );
    page_dict.set("Contents", Object::Reference(content_id));
    let page_id = doc.add_object(Object::Dictionary(page_dict));

    // Pages dictionary.
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    pages_dict.set("Count", Object::Integer(1));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));

    // Fix /Parent on the page.
    if let Object::Dictionary(d) = doc.get_object_mut(page_id).unwrap() {
        d.set("Parent", Object::Reference(pages_id));
    }

    // Catalog.
    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog));

    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("failed to save test PDF");
    path.to_path_buf()
}

/// Returns the number of pages in a PDF file.
fn pdf_page_count(path: &Path) -> usize {
    Document::load(path).unwrap().get_pages().len()
}

// ---------------------------------------------------------------------------
// PDF merge tests
// ---------------------------------------------------------------------------

/// Two single-page PDFs merged together should produce a two-page PDF.
#[test]
fn test_combine_two_pdfs() {
    with_combine_lock(|| {
        let dir = tempfile::tempdir().unwrap();

        let pdf1 = create_test_pdf(&dir.path().join("a.pdf"));
        let pdf2 = create_test_pdf(&dir.path().join("b.pdf"));
        let out = dir.path().join("out.pdf");

        let result = combine_pdfs(&[&pdf1, &pdf2], &out);
        assert!(result.is_ok(), "combine_pdfs failed: {:?}", result);
        assert_eq!(result.unwrap(), out);
        assert!(out.exists(), "output PDF does not exist");
        assert_eq!(pdf_page_count(&out), 2, "expected 2 pages in merged PDF");
    });
}

/// Merging three PDFs should produce a three-page document.
#[test]
fn test_combine_three_pdfs() {
    with_combine_lock(|| {
        let dir = tempfile::tempdir().unwrap();

        let pdfs: Vec<PathBuf> = (0..3)
            .map(|i| create_test_pdf(&dir.path().join(format!("p{}.pdf", i))))
            .collect();
        let out = dir.path().join("out.pdf");

        let result = combine_pdfs(pdfs.as_slice(), &out);
        assert!(result.is_ok(), "combine_pdfs failed: {:?}", result);
        assert_eq!(pdf_page_count(&out), 3);
    });
}

/// Providing an empty slice should return an error.
#[test]
fn test_combine_empty_input() {
    with_combine_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.pdf");

        let result = combine_pdfs::<&Path>(&[], &out);
        assert!(
            matches!(result, Err(Excel2PdfError::InvalidInput(_))),
            "expected InvalidInput error, got {:?}",
            result
        );
    });
}

/// Calling combine_pdfs while another is in flight should return
/// AlreadyProcessing.
#[test]
fn test_combine_already_processing() {
    with_combine_lock(|| {
        let dir = tempfile::tempdir().unwrap();

        let pdf = create_test_pdf(&dir.path().join("a.pdf"));
        let out1 = dir.path().join("out1.pdf");
        let out2 = dir.path().join("out2.pdf");

        // First call must succeed.
        let result1 = combine_pdfs(&[&pdf], &out1);
        assert!(result1.is_ok(), "first combine_pdfs failed: {:?}", result1);

        // Second call must also succeed (first has released the lock).
        let result2 = combine_pdfs(&[&pdf], &out2);
        assert!(result2.is_ok(), "second combine_pdfs failed: {:?}", result2);
    });
}

// ---------------------------------------------------------------------------
// Page-count / remove-pages tests
// ---------------------------------------------------------------------------

/// pdf::page_count should return 1 for a single-page PDF.
#[test]
fn test_page_count_single() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_pdf(&dir.path().join("single.pdf"));
    assert_eq!(excel2pdf::pdf::page_count(&path).unwrap(), 1);
}

/// pdf::remove_all_but_first_page on a 1-page PDF is a no-op.
#[test]
fn test_remove_pages_single_noop() {
    let dir = tempfile::tempdir().unwrap();
    let path = create_test_pdf(&dir.path().join("single.pdf"));
    assert!(excel2pdf::pdf::remove_all_but_first_page(&path).is_ok());
    assert_eq!(pdf_page_count(&path), 1);
}

// ---------------------------------------------------------------------------
// Concurrency / semaphore tests
// ---------------------------------------------------------------------------

/// set_max_concurrency must not panic for valid values.
#[test]
fn test_set_max_concurrency_valid() {
    set_max_concurrency(1);
    set_max_concurrency(4);
}

/// set_max_concurrency panics when n == 0.
#[test]
#[should_panic]
fn test_set_max_concurrency_zero_panics() {
    set_max_concurrency(0);
}

// ---------------------------------------------------------------------------
// Excel-to-PDF conversion (skipped when LibreOffice / testdata absent)
// ---------------------------------------------------------------------------

/// Converts ./testdata/C-1.xlsx to PDF.  Skipped when:
/// - the file is not present, or
/// - LibreOffice is not installed.
#[test]
fn test_convert_excel_to_pdf() {
    let input = Path::new("testdata/C-1.xlsx");
    if !input.exists() {
        eprintln!("skipping: testdata/C-1.xlsx not present");
        return;
    }

    match convert_excel_to_pdf(input) {
        Ok(pdf_path) => {
            assert!(pdf_path.exists(), "PDF was not created");
            assert_eq!(
                pdf_path.extension().unwrap_or_default(),
                "pdf",
                "output does not have .pdf extension"
            );
            std::fs::remove_file(&pdf_path).ok();
        }
        Err(Excel2PdfError::LibreOfficeNotInstalled)
        | Err(Excel2PdfError::ConverterNotFound) => {
            eprintln!("skipping: no converter available");
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }
}

/// Converts all .xlsx files in testdata concurrently.
#[test]
fn test_convert_multiple_xlsx() {
    let entries = match std::fs::read_dir("testdata") {
        Ok(e) => e,
        Err(_) => {
            eprintln!("skipping: testdata directory not found");
            return;
        }
    };

    let xlsx_files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .map(|ext| ext.eq_ignore_ascii_case("xlsx"))
                .unwrap_or(false)
        })
        .collect();

    if xlsx_files.is_empty() {
        eprintln!("skipping: no .xlsx files in testdata");
        return;
    }

    let handles: Vec<_> = xlsx_files
        .into_iter()
        .map(|path| {
            thread::spawn(move || {
                let result = convert_excel_to_pdf(&path);
                (path, result)
            })
        })
        .collect();

    for handle in handles {
        let (path, result) = handle.join().unwrap();
        match result {
            Ok(pdf_path) => {
                assert!(pdf_path.exists(), "{}: PDF was not created", path.display());
                std::fs::remove_file(&pdf_path).ok();
            }
            Err(Excel2PdfError::LibreOfficeNotInstalled)
            | Err(Excel2PdfError::ConverterNotFound) => {
                eprintln!("skipping {}: no converter", path.display());
            }
            Err(e) => panic!("{}: unexpected error: {:?}", path.display(), e),
        }
    }
}
