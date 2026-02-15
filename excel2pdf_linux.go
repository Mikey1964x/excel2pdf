//go:build linux || darwin
// +build linux darwin

package excel2pdf

func convertExcelToPdf(excelFile, pdfPath string) (pdfFile string, err error) {
	return convertExcelToPDFWithLibreOffice(excelFile, pdfPath)
}
