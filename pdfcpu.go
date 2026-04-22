package excel2pdf

import (
	"fmt"

	"github.com/pdfcpu/pdfcpu/pkg/api"
)

// combinePdfs merges pdfFiles into a single PDF written to outputPdfFile using
// pdfcpu and returns the output path.
func combinePdfs(pdfFiles []string, outputPdfFile string) (pdfFile string, err error) {
	err = api.MergeCreateFile(pdfFiles, outputPdfFile, false, nil)
	if err != nil {
		return "", err
	}
	return outputPdfFile, nil
}

// removeAllButFirstPage removes all pages after the first from the PDF at
// pdfFilePath, leaving only a single-page document. It is a no-op when the
// file already contains one page or fewer.
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
