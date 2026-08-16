package excel2pdf

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestConvertExcelToPdf verifies that ConvertExcelToPdf converts a single Excel
// file to PDF and writes the output alongside the source file.
//
// The test requires ./testdata/C-1.xlsx to be present; it is skipped when the
// file is missing so that the test suite can still pass in environments without
// test fixtures (e.g. CI without large binary assets).
//
// Assertions:
//   - No error is returned from ConvertExcelToPdf.
//   - The returned path has a .pdf suffix.
//   - The returned path points to testdata/C-1.pdf (absolute form).
//   - The generated file exists on disk and is non-empty.
//
// The generated PDF is removed by t.Cleanup after the test completes.
func TestConvertExcelToPdf(t *testing.T) {
	const input = "./testdata/C-1.xlsx"
	if _, err := os.Stat(input); os.IsNotExist(err) {
		t.Skipf("testdata not present: %s", input)
	}

	pdfFile, err := ConvertExcelToPdf(input)
	if err != nil {
		t.Fatalf("ConvertExcelToPdf(%q) error: %v", input, err)
	}
	t.Cleanup(func() {
		if err := os.Remove(pdfFile); err != nil && !os.IsNotExist(err) {
			t.Logf("cleanup: failed to remove %s: %v", pdfFile, err)
		}
	})

	if !strings.HasSuffix(pdfFile, ".pdf") {
		t.Errorf("expected .pdf suffix, got %q", pdfFile)
	}
	want := filepath.Join("testdata", "C-1.pdf")
	wantAbs, err := filepath.Abs(want)
	if err != nil {
		t.Fatalf("filepath.Abs(%q): %v", want, err)
	}
	if filepath.Clean(pdfFile) != filepath.Clean(wantAbs) {
		t.Errorf("expected output path %q, got %q", wantAbs, pdfFile)
	}
	if info, err := os.Stat(pdfFile); err != nil {
		t.Errorf("output file does not exist: %v", err)
	} else if info.Size() == 0 {
		t.Errorf("output PDF is empty")
	}
}
