package jar

import (
	"os"
	"path/filepath"
	"testing"
)

func TestVanillaProviderName(t *testing.T) {
	p := VanillaProvider{}
	if p.Name() != "vanilla" {
		t.Errorf("expected vanilla, got %s", p.Name())
	}
}

func TestListVersions(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	p := VanillaProvider{}
	versions, err := p.ListVersions(false)
	if err != nil {
		t.Fatalf("ListVersions failed: %v", err)
	}

	if len(versions) == 0 {
		t.Error("expected versions, got none")
	}

	for _, v := range versions {
		if v.Type != VersionTypeRelease {
			t.Errorf("expected only releases, got %s", v.Type)
		}
	}
}

func TestListVersionsWithSnapshots(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	p := VanillaProvider{}
	versions, err := p.ListVersions(true)
	if err != nil {
		t.Fatalf("ListVersions failed: %v", err)
	}

	hasSnapshot := false
	for _, v := range versions {
		if v.Type == VersionTypeSnapshot {
			hasSnapshot = true
			break
		}
	}
	if !hasSnapshot {
		t.Error("expected snapshots when includeSnapshots=true")
	}
}

func TestGetJavaVersion(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	p := VanillaProvider{}
	jv, err := p.GetJavaVersion("1.21.4")
	if err != nil {
		t.Fatalf("GetJavaVersion failed: %v", err)
	}

	if jv < 17 {
		t.Errorf("expected java >= 17 for 1.21.4, got %d", jv)
	}
}

func TestDownloadServer(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	tmpDir := t.TempDir()
	destPath := filepath.Join(tmpDir, "server.jar")

	p := VanillaProvider{}
	if err := p.DownloadServer("1.21.4", destPath); err != nil {
		t.Fatalf("DownloadServer failed: %v", err)
	}

	info, err := os.Stat(destPath)
	if err != nil {
		t.Fatalf("server.jar not found: %v", err)
	}

	if info.Size() < 1000000 {
		t.Errorf("server.jar too small: %d bytes", info.Size())
	}
}

func TestGetLatestRelease(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	release, err := GetLatestRelease()
	if err != nil {
		t.Fatalf("GetLatestRelease failed: %v", err)
	}
	if release == "" {
		t.Error("expected non-empty release")
	}
}
