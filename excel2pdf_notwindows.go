//go:build !windows
// +build !windows

package excel2pdf

// convertExcelToPdf converts excelFile to PDF on Linux and macOS using
// LibreOffice, which must be installed on the system.
func convertExcelToPdf(excelFile string) (pdfFile string, err error) {
	return convertExcelToPDFWithLibreOffice(excelFile)
}
