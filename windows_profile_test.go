//go:build windows
// +build windows

package excel2pdf

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
)

// memSnapshot holds a subset of runtime.MemStats captured at a point in time.
type memSnapshot struct {
	HeapAlloc  uint64 // bytes of allocated heap objects
	HeapSys    uint64 // bytes of heap memory obtained from the OS
	TotalAlloc uint64 // cumulative bytes allocated (never decreases)
	Sys        uint64 // total bytes of memory obtained from the OS
	NumGC      uint32 // number of completed GC cycles
}

func captureMemStats() memSnapshot {
	var ms runtime.MemStats
	runtime.ReadMemStats(&ms)
	return memSnapshot{
		HeapAlloc:  ms.HeapAlloc,
		HeapSys:    ms.HeapSys,
		TotalAlloc: ms.TotalAlloc,
		Sys:        ms.Sys,
		NumGC:      ms.NumGC,
	}
}

func toMB(b uint64) string {
	return fmt.Sprintf("%.2f MB", float64(b)/1024/1024)
}

// TestConvertExcelToPDFWithExcel_MemoryProfile converts all .xlsx files in
// ./testdata concurrently and reports heap memory usage before and after the
// conversions complete.
//
// The test is skipped when ./testdata contains no .xlsx files. It does not
// assert memory thresholds — it is intended to be run with -v to observe
// allocation patterns under concurrent load.
//
// Run with memory profiling:
//
//	go test -v -run TestConvertExcelToPDFWithExcel_MemoryProfile -memprofile mem.pprof ../
//	go tool pprof mem.pprof
func TestConvertExcelToPDFWithExcel_MemoryProfile(t *testing.T) {
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

	t.Logf("converting %d file(s) concurrently", len(inputs))

	// Force a GC before measuring so baseline is as clean as possible.
	runtime.GC()
	before := captureMemStats()

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

	after := captureMemStats()

	// Clean up generated PDFs.
	for _, r := range results {
		if r.pdfFile != "" {
			t.Cleanup(func() { os.Remove(r.pdfFile) })
		}
		if r.err != nil {
			t.Errorf("convertExcelToPDFWithExcel(%q) error: %v", r.input, r.err)
		}
	}

	// Report memory deltas.
	t.Logf("--- memory profile: concurrent Excel → PDF (%d files) ---", len(inputs))
	t.Logf("  HeapAlloc  before: %s  after: %s  delta: %s",
		toMB(before.HeapAlloc), toMB(after.HeapAlloc),
		toMB(after.HeapAlloc-before.HeapAlloc))
	t.Logf("  HeapSys    before: %s  after: %s  delta: %s",
		toMB(before.HeapSys), toMB(after.HeapSys),
		toMB(after.HeapSys-before.HeapSys))
	t.Logf("  TotalAlloc before: %s  after: %s  delta: %s",
		toMB(before.TotalAlloc), toMB(after.TotalAlloc),
		toMB(after.TotalAlloc-before.TotalAlloc))
	t.Logf("  Sys        before: %s  after: %s  delta: %s",
		toMB(before.Sys), toMB(after.Sys),
		toMB(after.Sys-before.Sys))
	t.Logf("  GC cycles  before: %d  after: %d  delta: %d",
		before.NumGC, after.NumGC, after.NumGC-before.NumGC)
}

// BenchmarkConvertExcelToPDFWithExcel_Concurrent benchmarks concurrent
// conversions of all .xlsx files in ./testdata.
//
// Run with:
//
//	go test -bench BenchmarkConvertExcelToPDFWithExcel_Concurrent -benchmem -memprofile mem.pprof ./...
func BenchmarkConvertExcelToPDFWithExcel_Concurrent(b *testing.B) {
	entries, err := os.ReadDir("./testdata")
	if err != nil {
		b.Fatalf("reading testdata dir: %v", err)
	}
	var inputs []string
	for _, e := range entries {
		if !e.IsDir() && strings.EqualFold(filepath.Ext(e.Name()), ".xlsx") {
			inputs = append(inputs, filepath.Join("./testdata", e.Name()))
		}
	}
	if len(inputs) == 0 {
		b.Skip("no .xlsx files found in testdata")
	}

	b.ReportAllocs()
	b.ResetTimer()

	for range b.N {
		results := make([]struct {
			pdfFile string
			err     error
		}, len(inputs))
		var wg sync.WaitGroup
		for i, input := range inputs {
			wg.Add(1)
			go func(idx int, path string) {
				defer wg.Done()
				pdfFile, err := convertExcelToPDFWithExcel(path)
				results[idx].pdfFile = pdfFile
				results[idx].err = err
			}(i, input)
		}
		wg.Wait()
		b.StopTimer()
		for _, r := range results {
			if r.err != nil {
				b.Errorf("convertExcelToPDFWithExcel error: %v", r.err)
			}
			if r.pdfFile != "" {
				os.Remove(r.pdfFile)
			}
		}
		b.StartTimer()
	}
}
