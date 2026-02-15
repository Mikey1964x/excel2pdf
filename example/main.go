package main

import (
	"fmt"

	"github.com/Mikey1964x/excel2pdf"
)

var excelPath = `C-1.xlsx`

func main() {
	fmt.Println(excel2pdf.ConvertExcelToPdf(excelPath))
}
