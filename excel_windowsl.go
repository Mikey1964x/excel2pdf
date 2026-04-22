//go:build windows
// +build windows

package excel2pdf

import (
	"log/slog"
	"path/filepath"
	"strings"

	"golang.org/x/sys/windows/registry"
)

// existExcel recursively searches the Windows registry under
// SOFTWARE\Microsoft\Office for an "Excel" subkey, indicating that Microsoft
// Excel is installed. names are the path segments appended to the base key on
// each recursive call.
func existExcel(names ...string) (bool, error) {
	const prefix = `SOFTWARE\Microsoft\Office`
	var keyPath = filepath.Join(append([]string{prefix}, names...)...)
	key, err := registry.OpenKey(registry.LOCAL_MACHINE, keyPath, registry.READ)
	if err != nil {
		slog.Error(`opening registry.LOCAL_MACHINE`, "error", err, "key_path", keyPath)
		return false, err
	}
	subkeys, err := key.ReadSubKeyNames(-1)
	if err != nil {
		slog.Error("reading  registry.LOCAL_MACHINE sub keys", "error", err, "key_path", keyPath)
		return false, err
	}
	for _, name := range subkeys {
		switch name {
		case "Excel":
			return true, nil
		case "ClickToRun", "Common", "Access",
			"ClickToRunStore", "Outlook", "PowerPoint",
			"Project", "SDXHelper", "Visio", "Word":
			continue
		}

		ok, err := existExcel(strings.TrimPrefix(keyPath, prefix), name)
		if err != nil {
			return false, err
		}
		if ok {
			return ok, nil
		}
	}
	return false, nil
}

// isExcelInstalled reports whether Microsoft Excel is installed on the current
// Windows machine by inspecting the registry.
func isExcelInstalled() (bool, error) { return existExcel() }
