// Package excel2pdf converts Excel (.xlsx) files to PDF.
//
// It supports Windows, Linux, and macOS. On Windows, LibreOffice is preferred if
// installed, with automatic fallback to Microsoft Excel. On Linux and macOS,
// LibreOffice is required.
//
// # Concurrency
//
// Up to [defaultMaxConcurrency] Excel-to-PDF conversions may run concurrently.
// Callers that exceed this limit will block until a slot becomes free.
// Call [SetMaxConcurrency] once at startup to adjust the limit before launching
// any goroutines.
//
// PDF merge operations ([CombinePdfs]) are exclusive: only one may run at a
// time. If another merge is already in progress, [ErrExcel2PdfIsProcessing] is
// returned immediately.
//
// # Example
//
//	excel2pdf.SetMaxConcurrency(4)
//
//	pdfPath, err := excel2pdf.ConvertExcelToPdf("report.xlsx")
//	if err != nil {
//		log.Fatal(err)
//	}
//	fmt.Println("PDF written to", pdfPath)
package excel2pdf

import (
	"errors"
	"fmt"
	"sync"
)

// defaultMaxConcurrency is the number of concurrent Excel-to-PDF conversions
// allowed when [SetMaxConcurrency] has not been called.
const defaultMaxConcurrency = 3

// convSem is a buffered channel used as a counting semaphore. Acquiring a slot
// means sending to the channel; releasing means receiving from it.
var convSem = make(chan struct{}, defaultMaxConcurrency)

// combineMu serialize [CombinePdfs] calls.
var combineMu sync.Mutex

// ErrExcel2PdfIsProcessing is returned by [CombinePdfs] when another merge
// operation is already in progress.
var ErrExcel2PdfIsProcessing = errors.New("a process is currently running using the resource")

// SetMaxConcurrency sets the maximum number of [ConvertExcelToPdf] calls that
// may run concurrently. It must be called before any concurrent conversions
// begin; it is not safe to call while conversions are in flight.
//
// n must be at least 1; values above the number of logical CPUs are unlikely to
// improve throughput and will increase memory pressure. The recommended range
// is 3–4 when Microsoft Excel or LibreOffice is the converter.
func SetMaxConcurrency(n int) {
	if n < 1 {
		panic(fmt.Sprintf("excel2pdf: SetMaxConcurrency: n must be >= 1, got %d", n))
	}
	convSem = make(chan struct{}, n)
}

// ConvertExcelToPdf converts an Excel file to PDF and returns the path of the
// generated PDF file.
//
// On Windows, LibreOffice is used when available; otherwise Microsoft Excel is
// used. On Linux and macOS, LibreOffice must be installed.
//
// Up to the concurrency limit set by [SetMaxConcurrency] conversions may run in
// parallel. If that limit is already reached, the call blocks until a slot
// becomes available.
func ConvertExcelToPdf(excelFile string) (pdfFile string, err error) {
	convSem <- struct{}{}        // acquire
	defer func() { <-convSem }() // release
	return convertExcelToPdf(excelFile)
}

// CombinePdfs merges one or more PDF files into a single PDF written to
// outputPdfFile and returns the output path.
//
// Only one merge may run at a time. If another call is already in progress,
// [ErrExcel2PdfIsProcessing] is returned immediately.
func CombinePdfs(pdfFiles []string, outputPdfFile string) (pdfFile string, err error) {
	if !combineMu.TryLock() {
		return "", ErrExcel2PdfIsProcessing
	}
	defer combineMu.Unlock()
	return combinePdfs(pdfFiles, outputPdfFile)
}
