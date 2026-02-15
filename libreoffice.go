package excel2pdf

import (
	"fmt"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

func convertExcelToPDFWithLibreOffice(excelFilePath, pdfPath string) (pdfFilePath string, err error) {
	libreOfficePath, err := findLibreOfficeBinPath()
	if err != nil {
		return "", err
	}
	cmd := exec.Command(
		libreOfficePath,
		"--headless",
		"--convert-to",
		"pdf", excelFilePath,
		"--outdir", pdfPath,
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
	var tmpPdfFilePath = filepath.Join(
		os.TempDir(),
		fmt.Sprintf("%s%s",
			strings.TrimSuffix(
				filepath.Base(excelFilePath),
				filepath.Ext(excelFilePath),
			),
			pdfSuffix,
		),
	)

	pdfFilePath = fmt.Sprintf("%s-%d%s",
		strings.TrimSuffix(tmpPdfFilePath, pdfSuffix),
		time.Now().Unix(),
		pdfSuffix,
	)
	if err := os.Rename(tmpPdfFilePath, pdfFilePath); err != nil {
		slog.Error("renaming pdf file", "error", err, "old_path", tmpPdfFilePath, "new_path", pdfFilePath)
		return tmpPdfFilePath, err
	}
	return pdfFilePath, nil
}
