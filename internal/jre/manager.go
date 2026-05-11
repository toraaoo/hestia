package jre

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/toraaoo/hestia/internal/config"
)

func jreDir() string {
	return filepath.Join(config.DefaultDir(), "jre")
}

func versionDir(majorVersion int) string {
	return filepath.Join(jreDir(), fmt.Sprintf("java-%d", majorVersion))
}

func javaBinaryPath(majorVersion int) string {
	return filepath.Join(versionDir(majorVersion), "bin", "java")
}

func IsInstalled(majorVersion int) bool {
	path := javaBinaryPath(majorVersion)
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return !info.IsDir() && info.Mode()&0111 != 0
}

func GetJRE(majorVersion int) (string, error) {
	if IsInstalled(majorVersion) {
		return javaBinaryPath(majorVersion), nil
	}
	if err := Download(majorVersion, nil); err != nil {
		return "", fmt.Errorf("download jre %d: %w", majorVersion, err)
	}
	path := javaBinaryPath(majorVersion)
	if _, err := os.Stat(path); err != nil {
		return "", fmt.Errorf("jre binary not found after download: %s", path)
	}
	return path, nil
}

func Verify(majorVersion int) error {
	path, err := GetJRE(majorVersion)
	if err != nil {
		return err
	}
	cmd := exec.Command(path, "-version")
	return cmd.Run()
}
