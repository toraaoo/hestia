package jre

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/toraaoo/hestia/internal/config"
)

func TestAdoptiumArch(t *testing.T) {
	arch := adoptiumArch()
	if arch == "" {
		t.Error("adoptiumArch returned empty string")
	}
	switch runtime.GOARCH {
	case "amd64":
		if arch != "x64" {
			t.Errorf("expected x64, got %s", arch)
		}
	case "arm64":
		if arch != "aarch64" {
			t.Errorf("expected aarch64, got %s", arch)
		}
	}
}

func TestAdoptiumOS(t *testing.T) {
	os := adoptiumOS()
	if os == "" {
		t.Error("adoptiumOS returned empty string")
	}
	if runtime.GOOS == "darwin" && os != "mac" {
		t.Errorf("expected mac, got %s", os)
	}
}

func TestDownloadURL(t *testing.T) {
	url := downloadURL(21)
	if !strings.Contains(url, "api.adoptium.net") {
		t.Errorf("expected adoptium url, got %s", url)
	}
	if !strings.Contains(url, "/21/") {
		t.Errorf("expected version 21 in url, got %s", url)
	}
}

func TestVersionDir(t *testing.T) {
	dir := versionDir(21)
	if !strings.HasSuffix(dir, filepath.Join("jre", "java-21")) {
		t.Errorf("unexpected version dir: %s", dir)
	}
}

func TestJavaBinaryPath(t *testing.T) {
	path := javaBinaryPath(21)
	expected := filepath.Join("bin", "java")
	if !strings.HasSuffix(path, expected) {
		t.Errorf("expected path ending with %s, got %s", expected, path)
	}
}

func TestIsInstalled_NotExists(t *testing.T) {
	if IsInstalled(9999) {
		t.Error("expected false for non-existent version")
	}
}

func TestDownloadAndVerify(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run download test")
	}

	tmpDir := t.TempDir()
	t.Setenv("HESTIA_DATA_DIR", tmpDir)

	version := 21
	path, err := GetJRE(version)
	if err != nil {
		t.Fatalf("GetJRE failed: %v", err)
	}

	if !strings.HasSuffix(path, filepath.Join("bin", "java")) {
		t.Errorf("unexpected java path: %s", path)
	}

	if _, err := os.Stat(path); err != nil {
		t.Errorf("java binary not found: %v", err)
	}

	if err := Verify(version); err != nil {
		t.Errorf("Verify failed: %v", err)
	}

	expectedDir := filepath.Join(tmpDir, "jre", "java-21")
	if !strings.HasPrefix(path, expectedDir) {
		t.Errorf("expected path under %s, got %s", expectedDir, path)
	}

	if !IsInstalled(version) {
		t.Error("IsInstalled should return true after download")
	}

	path2, err := GetJRE(version)
	if err != nil {
		t.Fatalf("second GetJRE failed: %v", err)
	}
	if path != path2 {
		t.Errorf("GetJRE returned different paths: %s vs %s", path, path2)
	}
}

func TestJreDir(t *testing.T) {
	dir := jreDir()
	expected := filepath.Join(config.DefaultDir(), "jre")
	if dir != expected {
		t.Errorf("expected %s, got %s", expected, dir)
	}
}
