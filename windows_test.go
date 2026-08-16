//go:build windows
// +build windows

package excel2pdf

import (
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

// TestConvertExcelToPDFWithExcel verifies that convertExcelToPDFWithExcel
// converts a single Excel file to PDF using Microsoft Excel via COM/OLE
// automation and writes the output alongside the source file.
//
// The test requires ./testdata/C-2.xlsx to be present; it is skipped when the
// file is missing so that the test suite can still pass in environments without
// test fixtures.
//
// Assertions:
//   - No error is returned from convertExcelToPDFWithExcel.
//   - The returned path has a .pdf suffix.
//   - The returned path points to testdata/C-2.pdf (absolute form).
//   - The generated file exists on disk and is non-empty.
//
// The generated PDF is removed by t.Cleanup after the test completes.
func TestConvertExcelToPDFWithExcel(t *testing.T) {
	const input = "./testdata/C-2.xlsx"
	if _, err := os.Stat(input); os.IsNotExist(err) {
		t.Skipf("testdata not present: %s", input)
	}

	pdfFile, err := convertExcelToPDFWithExcel(input)
	if err != nil {
		t.Fatalf("convertExcelToPDFWithExcel(%q) error: %v", input, err)
	}
	t.Cleanup(func() { os.Remove(pdfFile) })

	if !strings.HasSuffix(pdfFile, ".pdf") {
		t.Errorf("expected .pdf suffix, got %q", pdfFile)
	}
	want := filepath.Join("testdata", "C-2.pdf")
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

// TestConvertExcelToPDFWithExcel_Concurrent verifies that all Excel files found
// in ./testdata can be converted to PDF simultaneously using
// convertExcelToPDFWithExcel.
//
// The test is skipped when ./testdata contains no .xlsx files.
//
// Each conversion runs in its own goroutine. The test waits for all of them to
// finish, then asserts that every goroutine produced a non-empty PDF alongside
// its source file and returned no error.
//
// The generated PDFs are removed by t.Cleanup after the test completes.
func TestConvertExcelToPDFWithExcel_Concurrent(t *testing.T) {
	entries, err := os.ReadDir("./testdata")
	if err != nil {
		t.Fatalf("reading testdata dir: %v", err)
	}
	var inputs []string
	for _, e := range entries {
		if !e.IsDir() && strings.EqualFold(filepath.Ext(e.Name()), ".xlsx") {
			inputs = append(inputs, filepath.Join("./testdata", e.Name()))
		}
	}
	if len(inputs) == 0 {
		t.Skip("no .xlsx files found in testdata")
	}

	type result struct {
		input   string
		pdfFile string
		err     error
	}

	results := make([]result, len(inputs))
	var wg sync.WaitGroup
	for i, input := range inputs {
		wg.Add(1)
		go func(idx int, path string) {
			defer wg.Done()
			pdfFile, err := convertExcelToPDFWithExcel(path)
			results[idx] = result{input: path, pdfFile: pdfFile, err: err}
		}(i, input)
	}
	wg.Wait()

	for _, r := range results {
		r := r
		if r.err != nil {
			t.Errorf("convertExcelToPDFWithExcel(%q) error: %v", r.input, r.err)
			continue
		}
		t.Cleanup(func() { os.Remove(r.pdfFile) })

		if !strings.HasSuffix(r.pdfFile, ".pdf") {
			t.Errorf("%q: expected .pdf suffix, got %q", r.input, r.pdfFile)
		}
		wantAbs, err := filepath.Abs(
			filepath.Join(filepath.Dir(r.input), filepath.Base(r.input[:len(r.input)-len(filepath.Ext(r.input))]+".pdf")),
		)
		if err != nil {
			t.Errorf("%q: filepath.Abs error: %v", r.input, err)
			continue
		}
		if filepath.Clean(r.pdfFile) != filepath.Clean(wantAbs) {
			t.Errorf("%q: expected output path %q, got %q", r.input, wantAbs, r.pdfFile)
		}
		info, err := os.Stat(r.pdfFile)
		if err != nil {
			t.Errorf("%q: output file does not exist: %v", r.input, err)
		} else if info.Size() == 0 {
			t.Errorf("%q: output PDF is empty", r.input)
		}
	}
}

// TestConvertExcelToPDFWithExcel_Multiple verifies that all Excel files found
// in ./testdata can be converted to PDF simultaneously using
// convertExcelToPDFWithExcel.
//
// The test is skipped when ./testdata contains no .xlsx files.
//
// Each conversion runs in its own goroutine. The test waits for all of them to
// finish, then asserts that every goroutine produced a non-empty PDF alongside
// its source file and returned no error.
//
// The generated PDFs are removed by t.Cleanup after the test completes.
func TestConvertExcelToPDFWithExcel_Multiple(t *testing.T) {
	entries, err := os.ReadDir("./testdata")
	if err != nil {
		t.Fatalf("reading testdata dir: %v", err)
	}
	var inputs []string
	for _, e := range entries {
		if !e.IsDir() && strings.EqualFold(filepath.Ext(e.Name()), ".xlsx") {
			inputs = append(inputs, filepath.Join("./testdata", e.Name()))
		}
	}
	if len(inputs) == 0 {
		t.Skip("no .xlsx files found in testdata")
	}

	type result struct {
		input   string
		pdfFile string
		err     error
	}

	results := make([]result, len(inputs))
	var wg sync.WaitGroup
	for i, input := range inputs {
		wg.Add(1)
		go func(idx int, path string) {
			defer wg.Done()
			pdfFile, err := convertExcelToPDFWithExcel(path)
			results[idx] = result{input: path, pdfFile: pdfFile, err: err}
		}(i, input)
	}
	wg.Wait()

	for _, r := range results {
		r := r
		if r.err != nil {
			t.Errorf("convertExcelToPDFWithExcel(%q) error: %v", r.input, r.err)
			continue
		}
		t.Cleanup(func() { os.Remove(r.pdfFile) })

		if !strings.HasSuffix(r.pdfFile, ".pdf") {
			t.Errorf("%q: expected .pdf suffix, got %q", r.input, r.pdfFile)
		}
		wantAbs, err := filepath.Abs(
			filepath.Join(filepath.Dir(r.input), filepath.Base(r.input[:len(r.input)-len(filepath.Ext(r.input))]+".pdf")),
		)
		if err != nil {
			t.Errorf("%q: filepath.Abs error: %v", r.input, err)
			continue
		}
		if filepath.Clean(r.pdfFile) != filepath.Clean(wantAbs) {
			t.Errorf("%q: expected output path %q, got %q", r.input, wantAbs, r.pdfFile)
		}
		info, err := os.Stat(r.pdfFile)
		if err != nil {
			t.Errorf("%q: output file does not exist: %v", r.input, err)
		} else if info.Size() == 0 {
			t.Errorf("%q: output PDF is empty", r.input)
		}
	}
}
