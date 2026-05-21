package backup

import (
	"archive/tar"
	"compress/gzip"
	"encoding/json"
	"errors"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/toraaoo/hestia/internal/server"
)

func TestCreateIncludesUnifiedBackupSources(t *testing.T) {
	store, cfg := newTestStore(t)
	worldDir := filepath.Join(store.DataDir(cfg.Name), cfg.World.Name)

	writeTestFile(t, filepath.Join(worldDir, "level.dat"), "world")
	writeTestFile(t, filepath.Join(worldDir, "region", "r.0.0.mca"), "region")
	writeTestFile(t, filepath.Join(worldDir, "session.lock"), "lock")
	writeTestFile(t, filepath.Join(store.DataDir(cfg.Name), "server.properties"), "motd=test")
	writeTestFile(t, filepath.Join(store.DataDir(cfg.Name), "plugins", "plugin.jar"), "plugin")
	writeTestFile(t, filepath.Join(store.DataDir(cfg.Name), "mods", "mod.jar"), "mod")

	svc := NewService(store, nil)
	svc.now = func() time.Time { return time.Date(2026, 5, 20, 10, 0, 0, 0, time.UTC) }

	info, err := svc.Create(Options{ServerName: cfg.Name})
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	if !strings.HasPrefix(info.Name, "backup-") {
		t.Fatalf("expected backup-* name, got %s", info.Name)
	}

	entries := readArchiveEntries(t, info.Path)
	assertArchiveEntry(t, entries, filepath.ToSlash(filepath.Join(cfg.World.Name, "level.dat")), "world")
	assertArchiveEntry(t, entries, filepath.ToSlash(filepath.Join(cfg.World.Name, "region", "r.0.0.mca")), "region")
	assertArchiveEntry(t, entries, "server.properties", "motd=test")
	assertArchiveEntry(t, entries, filepath.ToSlash(filepath.Join("plugins", "plugin.jar")), "plugin")
	assertArchiveEntry(t, entries, filepath.ToSlash(filepath.Join("mods", "mod.jar")), "mod")

	if _, ok := entries[filepath.ToSlash(filepath.Join(cfg.World.Name, "session.lock"))]; ok {
		t.Fatalf("session.lock should be excluded from backups")
	}
}

func TestCreateSucceedsWithoutOptionalBackupPaths(t *testing.T) {
	store, cfg := newTestStore(t)
	writeTestFile(t, filepath.Join(store.DataDir(cfg.Name), cfg.World.Name, "level.dat"), "world")

	svc := NewService(store, nil)
	info, err := svc.Create(Options{ServerName: cfg.Name})
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	entries := readArchiveEntries(t, info.Path)
	assertArchiveEntry(t, entries, filepath.ToSlash(filepath.Join(cfg.World.Name, "level.dat")), "world")
	if _, ok := entries[filepath.ToSlash(filepath.Join("plugins", "plugin.jar"))]; ok {
		t.Fatalf("unexpected plugin entry in archive")
	}
	if _, ok := entries[filepath.ToSlash(filepath.Join("mods", "mod.jar"))]; ok {
		t.Fatalf("unexpected mod entry in archive")
	}
}

func TestRestoreRestoresUnifiedBackupTargets(t *testing.T) {
	store, cfg := newTestStore(t)
	dataDir := store.DataDir(cfg.Name)
	worldDir := filepath.Join(dataDir, cfg.World.Name)

	writeTestFile(t, filepath.Join(worldDir, "level.dat"), "world")
	writeTestFile(t, filepath.Join(dataDir, "plugins", "kept.jar"), "plugin")
	writeTestFile(t, filepath.Join(dataDir, "mods", "kept.jar"), "mod")
	writeTestFile(t, filepath.Join(dataDir, "server.properties"), "motd=before")

	svc := NewService(store, nil)
	info, err := svc.Create(Options{ServerName: cfg.Name})
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	writeTestFile(t, filepath.Join(worldDir, "level.dat"), "mutated")
	writeTestFile(t, filepath.Join(dataDir, "plugins", "extra.jar"), "stale")
	writeTestFile(t, filepath.Join(dataDir, "mods", "extra.jar"), "stale")
	writeTestFile(t, filepath.Join(dataDir, "server.properties"), "motd=after")

	if err := svc.Restore(cfg.Name, info.Name); err != nil {
		t.Fatalf("Restore failed: %v", err)
	}

	assertFileContent(t, filepath.Join(worldDir, "level.dat"), "world")
	assertFileContent(t, filepath.Join(dataDir, "plugins", "kept.jar"), "plugin")
	assertFileContent(t, filepath.Join(dataDir, "mods", "kept.jar"), "mod")
	assertFileContent(t, filepath.Join(dataDir, "server.properties"), "motd=before")

	if _, err := os.Stat(filepath.Join(dataDir, "plugins", "extra.jar")); !os.IsNotExist(err) {
		t.Fatalf("expected stale plugin file to be removed, got err=%v", err)
	}
	if _, err := os.Stat(filepath.Join(dataDir, "mods", "extra.jar")); !os.IsNotExist(err) {
		t.Fatalf("expected stale mod file to be removed, got err=%v", err)
	}
}

func TestArchivePathNormalizesWindowsSeparators(t *testing.T) {
	got := archivePath(`world\region\r.0.0.mca`)
	if got != "world/region/r.0.0.mca" {
		t.Fatalf("expected slash-normalized archive path, got %q", got)
	}
}

func TestCreateFailsOnUnexpectedReadError(t *testing.T) {
	store, cfg := newTestStore(t)
	worldDir := filepath.Join(store.DataDir(cfg.Name), cfg.World.Name)

	writeTestFile(t, filepath.Join(worldDir, "level.dat"), "world")
	failPath := filepath.Join(worldDir, "fail.dat")
	writeTestFile(t, failPath, "boom")

	origOpen := openSourceFile
	openSourceFile = func(name string) (*os.File, error) {
		if name == failPath {
			return nil, errors.New("forced open failure")
		}
		return os.Open(name)
	}
	defer func() { openSourceFile = origOpen }()

	svc := NewService(store, nil)
	_, err := svc.Create(Options{ServerName: cfg.Name})
	if err == nil || !strings.Contains(err.Error(), "forced open failure") {
		t.Fatalf("expected forced open failure, got %v", err)
	}
}

func TestListIncludesLegacyBackups(t *testing.T) {
	store, cfg := newTestStore(t)
	backupDir := store.BackupsDir(cfg.Name)
	if err := os.MkdirAll(backupDir, 0755); err != nil {
		t.Fatalf("MkdirAll failed: %v", err)
	}

	legacyWithMeta := filepath.Join(backupDir, "world-20240102-030405.tar.gz")
	writeTestFile(t, legacyWithMeta, "a")

	meta := map[string]any{
		"name":       filepath.Base(legacyWithMeta),
		"path":       legacyWithMeta,
		"type":       "world",
		"size":       1,
		"created_at": time.Date(2024, 1, 2, 3, 4, 5, 0, time.UTC),
		"world_name": cfg.World.Name,
		"version":    cfg.Version,
	}
	metaData, err := json.Marshal(meta)
	if err != nil {
		t.Fatalf("Marshal failed: %v", err)
	}
	writeTestFile(t, legacyWithMeta+".json", string(metaData))

	legacyWithoutMeta := filepath.Join(backupDir, "full-20240101-010101.tar.gz")
	writeTestFile(t, legacyWithoutMeta, "b")
	modTime := time.Date(2024, 1, 1, 1, 1, 1, 0, time.UTC)
	if err := os.Chtimes(legacyWithoutMeta, modTime, modTime); err != nil {
		t.Fatalf("Chtimes failed: %v", err)
	}

	svc := NewService(store, nil)
	backups, err := svc.List(cfg.Name)
	if err != nil {
		t.Fatalf("List failed: %v", err)
	}

	if len(backups) != 2 {
		t.Fatalf("expected 2 backups, got %d", len(backups))
	}
	if backups[0].Name != filepath.Base(legacyWithMeta) {
		t.Fatalf("expected newest backup first, got %s", backups[0].Name)
	}
	if backups[0].WorldName != cfg.World.Name {
		t.Fatalf("expected world name %s, got %s", cfg.World.Name, backups[0].WorldName)
	}
	if backups[1].Name != filepath.Base(legacyWithoutMeta) {
		t.Fatalf("expected second backup %s, got %s", filepath.Base(legacyWithoutMeta), backups[1].Name)
	}
}

func newTestStore(t *testing.T) (*server.Store, *server.Config) {
	t.Helper()

	root := t.TempDir()
	t.Setenv("HESTIA_DATA_DIR", root)

	store := server.NewStore(filepath.Join(root, "servers"))
	cfg, err := store.Create("testserver", "1.21.4")
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	return store, cfg
}

func writeTestFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		t.Fatalf("MkdirAll failed: %v", err)
	}
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}
}

func readArchiveEntries(t *testing.T, archivePath string) map[string]string {
	t.Helper()

	f, err := os.Open(archivePath)
	if err != nil {
		t.Fatalf("Open failed: %v", err)
	}
	defer func() { _ = f.Close() }()

	gr, err := gzip.NewReader(f)
	if err != nil {
		t.Fatalf("NewReader failed: %v", err)
	}
	defer func() { _ = gr.Close() }()

	tr := tar.NewReader(gr)
	entries := make(map[string]string)

	for {
		header, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			t.Fatalf("Next failed: %v", err)
		}
		if header.Typeflag != tar.TypeReg {
			continue
		}

		data, err := io.ReadAll(tr)
		if err != nil {
			t.Fatalf("ReadAll failed: %v", err)
		}
		entries[filepath.ToSlash(header.Name)] = string(data)
	}

	return entries
}

func assertArchiveEntry(t *testing.T, entries map[string]string, name, want string) {
	t.Helper()
	got, ok := entries[name]
	if !ok {
		t.Fatalf("expected archive entry %s", name)
	}
	if got != want {
		t.Fatalf("expected archive entry %s to equal %q, got %q", name, want, got)
	}
}

func assertFileContent(t *testing.T, path, want string) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("ReadFile failed: %v", err)
	}
	if string(data) != want {
		t.Fatalf("expected %s to equal %q, got %q", path, want, string(data))
	}
}
