// Package excel2pdf converts Excel (.xlsx) files to PDF.
//
// It supports Windows, Linux, and macOS. On Windows, LibreOffice is preferred if
// installed, with automatic fallback to Microsoft Excel. On Linux and macOS,
// LibreOffice is required.
//
// All operations are protected by a mutex so that only one conversion or merge
// can run at a time. Callers that attempt a second concurrent operation will
// receive [ErrExcel2PdfIsProcessing].
//
// # Example
//
//	pdfPath, err := excel2pdf.ConvertExcelToPdf("report.xlsx")
//	if err != nil {
//		log.Fatal(err)
//	}
//	fmt.Println("PDF written to", pdfPath)
package excel2pdf

import (
	"errors"
	"sync"
)

var (
	mutex sync.Mutex
)

// ErrExcel2PdfIsProcessing is returned by [ConvertExcelToPdf] or [CombinePdfs]
// when another operation is already in progress.
var ErrExcel2PdfIsProcessing = errors.New("a process is currently running using the resource")

// ConvertExcelToPdf converts an Excel file to PDF and returns the path of the
// generated PDF file.
//
// On Windows, LibreOffice is used when available; otherwise Microsoft Excel is
// used. On Linux and macOS, LibreOffice must be installed.
//
// Only one conversion may run at a time. If another call is already in
// progress, [ErrExcel2PdfIsProcessing] is returned immediately.
func ConvertExcelToPdf(excelFile string) (pdfFile string, err error) {
	if !mutex.TryLock() {
		return "", ErrExcel2PdfIsProcessing
	}
	defer mutex.Unlock()
	return convertExcelToPdf(excelFile)
}

// CombinePdfs merges one or more PDF files into a single PDF written to
// outputPdfFile and returns the output path.
//
// Only one operation may run at a time. If another call is already in
// progress, [ErrExcel2PdfIsProcessing] is returned immediately.
func CombinePdfs(pdfFiles []string, outputPdfFile string) (pdfFile string, err error) {
	if !mutex.TryLock() {
		return "", ErrExcel2PdfIsProcessing
	}
	defer mutex.Unlock()
	return combinePdfs(pdfFiles, outputPdfFile)
}
