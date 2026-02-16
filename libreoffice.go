package excel2pdf

import (
	"fmt"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

func convertExcelToPDFWithLibreOffice(excelFilePath string) (pdfFilePath string, err error) {
	libreOfficePath, err := findLibreOfficeBinPath()
	if err != nil {
		return "", err
	}
	// make excelFilePath absolute path
	excelFilePath, err = filepath.Abs(excelFilePath)
	if err != nil {
		slog.Error("get absolute path", "error", err, "excel_file_path", excelFilePath)
		return "", fmt.Errorf("failed to get absolute path: %w", err)
	}
	cmd := exec.Command(
		libreOfficePath,
		"--headless",
		"--convert-to", "pdf",
		"--outdir", filepath.Dir(excelFilePath),
		excelFilePath,
	)

	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		slog.Error("libreoffice running", "error", err, "libre_office_path", libreOfficePath)
		return "", fmt.Errorf("failed to convert file: %w", err)
	}
	if cmd.Err != nil {
		slog.Error("libreoffice command", "error", err, "libre_office_path", libreOfficePath)
	}

	const pdfSuffix = ".pdf"
	pdfFilePath = filepath.Join(
		filepath.Dir(excelFilePath),
		fmt.Sprintf("%s%s",
			strings.TrimSuffix(
				filepath.Base(excelFilePath),
				filepath.Ext(excelFilePath),
			),
			pdfSuffix,
		),
	)

	// open the generated PDF file and delete all but the first page. Then save
	// the modified PDF file with the same name, overwriting the original PDF file.
	if err := removeAllButFirstPage(pdfFilePath); err != nil {
		slog.Error("remove all but first page", "error", err, "pdf_file_path", pdfFilePath)
		return "", fmt.Errorf("failed to remove all but first page: %w", err)
	}

	return pdfFilePath, nil
}
