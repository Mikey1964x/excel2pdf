package excel2pdf

import (
	"fmt"

	"github.com/pdfcpu/pdfcpu/pkg/api"
)

func combinePdfs(pdfFiles []string, outputPdfFile string) (pdfFile string, err error) {
	err = api.MergeCreateFile(pdfFiles, outputPdfFile, false, nil)
	if err != nil {
		return "", err
	}
	return outputPdfFile, nil
}

func removeAllButFirstPage(pdfFilePath string) error {
	pageCount, err := api.PageCountFile(pdfFilePath)
	if err != nil {
		return fmt.Errorf("failed to get page count: %w", err)
	}
	if pageCount <= 1 {
		return nil
	}
	// Remove pages 2 through the last page
	selectedPages := []string{fmt.Sprintf("2-%d", pageCount)}
	if err := api.RemovePagesFile(pdfFilePath, "", selectedPages, nil); err != nil {
		return fmt.Errorf("failed to remove pages: %w", err)
	}
	return nil
}
