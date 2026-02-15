package main

import (
	"fmt"

	"github.com/Mikey1964x/excel2pdf"
)

var excelPath = `C-1.xlsx`
var pdfPath = `C-1.pdf`

func main() {
	fmt.Println(excel2pdf.ConvertExcelToPdf(excelPath, pdfPath))
}
