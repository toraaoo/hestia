package jar_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/toraaoo/hestia/internal/jar/loaders"
)

func TestVanillaProviderName(t *testing.T) {
	registry := loaders.NewRegistry(nil, nil)
	p, err := registry.GetLoader("vanilla")
	if err != nil {
		t.Fatalf("GetProvider failed: %v", err)
	}
	if p.Name() != "vanilla" {
		t.Errorf("expected vanilla, got %s", p.Name())
	}
}

func TestListVersions(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	registry := loaders.NewRegistry(nil, nil)
	p, _ := registry.GetLoader("vanilla")
	versions, err := p.ListVersions(false)
	if err != nil {
		t.Fatalf("ListVersions failed: %v", err)
	}

	if len(versions) == 0 {
		t.Error("expected versions, got none")
	}

	for _, v := range versions {
		if v.Type != "release" {
			t.Errorf("expected only releases, got %s", v.Type)
		}
	}
}

func TestListVersionsWithSnapshots(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	registry := loaders.NewRegistry(nil, nil)
	p, _ := registry.GetLoader("vanilla")
	versions, err := p.ListVersions(true)
	if err != nil {
		t.Fatalf("ListVersions failed: %v", err)
	}

	hasSnapshot := false
	for _, v := range versions {
		if v.Type == "snapshot" {
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

	registry := loaders.NewRegistry(nil, nil)
	p, _ := registry.GetLoader("vanilla")
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

	registry := loaders.NewRegistry(nil, nil)
	p, _ := registry.GetLoader("vanilla")
	if err := p.DownloadServer("1.21.4", destPath, nil); err != nil {
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

func TestGetLatestVanillaVersions(t *testing.T) {
	if os.Getenv("HESTIA_INTEGRATION_TEST") == "" {
		t.Skip("set HESTIA_INTEGRATION_TEST=1 to run")
	}

	registry := loaders.NewRegistry(nil, nil)
	p, err := registry.GetLoader("vanilla")
	if err != nil {
		t.Fatalf("GetProvider failed: %v", err)
	}

	release, snapshot, err := registry.ResolveLatestVersions(p)
	if err != nil {
		t.Fatalf("ResolveLatestVersions failed: %v", err)
	}
	if release == "" {
		t.Error("expected non-empty release")
	}
	if snapshot == "" {
		t.Error("expected non-empty snapshot")
	}
}
