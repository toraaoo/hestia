package jre

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"

	"github.com/toraaoo/hestia/internal/progress"
)

type Manager struct {
	rootDir    string
	downloader *Downloader
}

func NewManager(rootDir string, downloader *Downloader) *Manager {
	if downloader == nil {
		downloader = NewDownloader(nil)
	}
	return &Manager{rootDir: rootDir, downloader: downloader}
}

func (m *Manager) IsInstalled(majorVersion int) bool {
	path := m.JavaBinaryPath(majorVersion)
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	if info.IsDir() {
		return false
	}
	if runtime.GOOS == "windows" {
		return true
	}
	return info.Mode()&0111 != 0
}

func (m *Manager) Get(majorVersion int, cb progress.Callback) (string, error) {
	if m.IsInstalled(majorVersion) {
		return m.JavaBinaryPath(majorVersion), nil
	}
	if err := m.Download(majorVersion, cb); err != nil {
		return "", fmt.Errorf("download jre %d: %w", majorVersion, err)
	}
	path := m.JavaBinaryPath(majorVersion)
	if _, err := os.Stat(path); err != nil {
		return "", fmt.Errorf("jre binary not found after download: %s", path)
	}
	return path, nil
}

func (m *Manager) Download(majorVersion int, cb progress.Callback) error {
	return m.downloader.Download(majorVersion, m.versionDir(majorVersion), cb)
}

func (m *Manager) Verify(majorVersion int) error {
	path, err := m.Get(majorVersion, nil)
	if err != nil {
		return err
	}
	cmd := exec.Command(path, "-version")
	return cmd.Run()
}

func (m *Manager) JavaBinaryPath(majorVersion int) string {
	b := "java"
	if runtime.GOOS == "windows" {
		b = "java.exe"
	}
	return filepath.Join(m.versionDir(majorVersion), "bin", b)
}

func (m *Manager) versionDir(majorVersion int) string {
	return filepath.Join(m.rootDir, fmt.Sprintf("java-%d", majorVersion))
}
