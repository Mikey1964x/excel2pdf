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

	return pdfFilePath, nil
}
