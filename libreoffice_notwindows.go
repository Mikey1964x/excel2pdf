//go:build !windows
// +build !windows

package excel2pdf

import (
	"errors"
	"log/slog"
	"os"
	"os/exec"
	"strings"
)

// ErrLibreofficeNotInstalled is returned on Linux and macOS when LibreOffice
// cannot be located on the system.
var ErrLibreofficeNotInstalled = errors.New("LibreOffice is not installed")

// findLibreOffice resolves the path to the "libreoffice" executable using
// `which` and returns it, or ErrLibreofficeNotInstalled if not found.
func findLibreOffice() (string, error) {
	cmd := exec.Command("which", "libreoffice")
	out, err := cmd.CombinedOutput()
	if err != nil {
		slog.Error("reading output `which libreoffice`", "error", err)
		return "", ErrLibreofficeNotInstalled
	}
	var libreofficePath = strings.TrimSpace(string(out))
	return libreofficePath, nil
}

// findlibreoffice24_8 resolves the path to the versioned "libreoffice24.8"
// executable using `which` and returns it, or ErrLibreofficeNotInstalled if
// not found.
func findlibreoffice24_8() (string, error) {
	cmd := exec.Command("which", "libreoffice24.8")
	out, err := cmd.CombinedOutput()
	if err != nil {
		slog.Error("reading output `which libreoffice`", "error", err)
		return "", ErrLibreofficeNotInstalled
	}
	var libreofficePath = strings.TrimSpace(string(out))
	return libreofficePath, nil
}

// findLibreOfficeBinPath returns the path to the LibreOffice binary.
//
// Resolution order:
//  1. The LIBREOFFICE_PATH environment variable, if set.
//  2. The "libreoffice" executable found via `which`.
//  3. The versioned "libreoffice24.8" executable found via `which`.
//
// Returns ErrLibreofficeNotInstalled if none of the above can be resolved.
func findLibreOfficeBinPath() (string, error) {
	value, ok := os.LookupEnv("LIBREOFFICE_PATH")
	if ok {
		return value, nil
	}
	libreofficePath, err := findLibreOffice()
	if err != nil {
		return "", err
	}
	if libreofficePath == "" {
		libreofficePath, err = findlibreoffice24_8()
		if err != nil {
			return "", err
		}
	}
	if libreofficePath == "" {
		return libreofficePath, ErrLibreofficeNotInstalled
	}
	return libreofficePath, nil
}
