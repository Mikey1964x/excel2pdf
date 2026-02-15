package main

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"

	"github.com/Mikey1964x/excel2pdf"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: example <directory>")
		os.Exit(1)
	}
	dir := os.Args[1]

	// Find all files matching C-<integer>.xlsx
	pattern := regexp.MustCompile(`^C-(\d+)\.xlsx$`)
	entries, err := os.ReadDir(dir)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error reading directory: %v\n", err)
		os.Exit(1)
	}

	type numberedFile struct {
		num  int
		path string
	}
	var files []numberedFile
	for _, e := range entries {
		if e.IsDir() {
			continue
		}
		matches := pattern.FindStringSubmatch(e.Name())
		if matches != nil {
			n, _ := strconv.Atoi(matches[1])
			files = append(files, numberedFile{num: n, path: filepath.Join(dir, e.Name())})
		}
	}
	if len(files) == 0 {
		fmt.Fprintln(os.Stderr, "No C-*.xlsx files found")
		os.Exit(1)
	}

	// Sort by the integer portion
	sort.Slice(files, func(i, j int) bool { return files[i].num < files[j].num })

	// Convert each Excel file to PDF
	var pdfFiles []string
	for _, f := range files {
		pdfFile, err := excel2pdf.ConvertExcelToPdf(f.path)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error converting %s: %v\n", f.path, err)
			os.Exit(1)
		}
		fmt.Printf("Converted: %s -> %s\n", f.path, pdfFile)
		pdfFiles = append(pdfFiles, pdfFile)
	}

	// Combine all PDFs into Timesheets.pdf
	outputPdf := filepath.Join(dir, "Timesheets.pdf")
	_, err = excel2pdf.CombinePdfs(pdfFiles, outputPdf)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error combining PDFs: %v\n", err)
		os.Exit(1)
	}
	fmt.Printf("Combined PDF: %s\n", outputPdf)

	// Delete the individual C-*.pdf files
	for _, pdfFile := range pdfFiles {
		if err := os.Remove(pdfFile); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: failed to delete %s: %v\n", pdfFile, err)
		} else {
			fmt.Printf("Deleted: %s\n", pdfFile)
		}
	}
}
