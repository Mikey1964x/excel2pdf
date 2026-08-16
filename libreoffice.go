package excel2pdf

import (
	"fmt"
	"log/slog"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// convertExcelToPDFWithLibreOffice converts excelFilePath to PDF by invoking
// LibreOffice in headless mode. The PDF is written to the same directory as
// the source file and only the first page is retained. The path of the
// generated PDF is returned.
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

	// Each concurrent LibreOffice process needs its own user profile directory.
	// Without this, multiple headless instances will collide on the same lock
	// file and fail to start.
	profileDir, err := os.MkdirTemp("", "lo_profile_*")
	if err != nil {
		return "", fmt.Errorf("failed to create libreoffice profile dir: %w", err)
	}
	defer func() {
		if err := os.RemoveAll(profileDir); err != nil {
			slog.Warn("failed to remove libreoffice profile dir", "error", err, "dir", profileDir)
		}
	}()

	// Build a file:// URL from the profile path (works on Windows and Linux).
	slashed := filepath.ToSlash(profileDir)
	if !strings.HasPrefix(slashed, "/") {
		slashed = "/" + slashed // Windows: C:/... → /C:/...
	}
	profileURL := "file://" + slashed

	cmd := exec.Command( //nolint:gosec // G204: libreOfficePath is resolved internally via findLibreOfficeBinPath, not user input
		libreOfficePath,
		"--headless",
		"--norestore",
		"--env:UserInstallation="+profileURL,
		"--convert-to", "pdf",
		"--outdir", filepath.Dir(excelFilePath),
		excelFilePath,
	) //#nosec G204

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
