package server

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestDefaultConfig(t *testing.T) {
	cfg := DefaultConfig("test", "1.21.4")

	if cfg.ConfigVersion != CurrentConfigVersion {
		t.Errorf("expected config version %d, got %d", CurrentConfigVersion, cfg.ConfigVersion)
	}
	if cfg.Name != "test" {
		t.Errorf("expected name test, got %s", cfg.Name)
	}
	if cfg.Version != "1.21.4" {
		t.Errorf("expected version 1.21.4, got %s", cfg.Version)
	}
	if cfg.Memory != "2G" {
		t.Errorf("expected memory 2G, got %s", cfg.Memory)
	}
	if !cfg.RCON.Enabled {
		t.Error("expected RCON enabled by default")
	}
	if cfg.RCON.Password == "" {
		t.Error("expected RCON password to be generated")
	}
}

func TestResolvePorts(t *testing.T) {
	cfg := DefaultConfig("test", "1.21.4")
	if err := cfg.ResolvePorts(); err != nil {
		t.Fatalf("ResolvePorts failed: %v", err)
	}

	if cfg.Port < 25565 || cfg.Port > 25600 {
		t.Errorf("port out of range: %d", cfg.Port)
	}
	if cfg.RCON.Port < 25575 || cfg.RCON.Port > 25600 {
		t.Errorf("rcon port out of range: %d", cfg.RCON.Port)
	}
}

func TestGenerateProperties(t *testing.T) {
	cfg := DefaultConfig("test", "1.21.4")
	cfg.Port = 25565
	cfg.RCON.Port = 25575

	props := cfg.GenerateProperties()

	if !strings.Contains(props, "server-port=25565") {
		t.Error("missing server-port")
	}
	if !strings.Contains(props, "rcon.port=25575") {
		t.Error("missing rcon.port")
	}
	if !strings.Contains(props, "enable-rcon=true") {
		t.Error("missing enable-rcon")
	}
	if !strings.Contains(props, "gamemode=survival") {
		t.Error("missing gamemode")
	}
}

func TestCreateAndDelete(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(filepath.Join(tmpDir, "servers"))

	cfg, err := store.Create("testserver", "1.21.4")
	if err != nil {
		t.Fatalf("Create failed: %v", err)
	}

	if cfg.Name != "testserver" {
		t.Errorf("expected name testserver, got %s", cfg.Name)
	}

	if !store.Exists("testserver") {
		t.Error("server should exist after Create")
	}

	configPath := filepath.Join(tmpDir, "servers", "testserver", "hestia.toml")
	if _, err := os.Stat(configPath); err != nil {
		t.Errorf("hestia.toml not found: %v", err)
	}

	propsPath := filepath.Join(tmpDir, "servers", "testserver", "data", "server.properties")
	if _, err := os.Stat(propsPath); err != nil {
		t.Errorf("server.properties not found: %v", err)
	}

	_, err = store.Create("testserver", "1.21.4")
	if err == nil {
		t.Error("expected error when creating duplicate server")
	}

	if err := store.Delete("testserver"); err != nil {
		t.Fatalf("Delete failed: %v", err)
	}

	if store.Exists("testserver") {
		t.Error("server should not exist after Delete")
	}
}

func TestList(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(filepath.Join(tmpDir, "servers"))

	names, err := store.List()
	if err != nil {
		t.Fatalf("List failed: %v", err)
	}
	if len(names) != 0 {
		t.Errorf("expected empty list, got %v", names)
	}

	_, _ = store.Create("server1", "1.21.4")
	_, _ = store.Create("server2", "1.20.4")

	names, err = store.List()
	if err != nil {
		t.Fatalf("List failed: %v", err)
	}
	if len(names) != 2 {
		t.Errorf("expected 2 servers, got %d", len(names))
	}
}

func TestLoadConfig(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(filepath.Join(tmpDir, "servers"))

	_, _ = store.Create("loadtest", "1.21.4")

	cfg, err := store.LoadConfig("loadtest")
	if err != nil {
		t.Fatalf("LoadConfig failed: %v", err)
	}

	if cfg.Name != "loadtest" {
		t.Errorf("expected name loadtest, got %s", cfg.Name)
	}
	if cfg.ConfigVersion != CurrentConfigVersion {
		t.Errorf("expected config version %d, got %d", CurrentConfigVersion, cfg.ConfigVersion)
	}
	if cfg.Version != "1.21.4" {
		t.Errorf("expected version 1.21.4, got %s", cfg.Version)
	}
}

func TestLoadConfigMigratesLegacyBackupType(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(filepath.Join(tmpDir, "servers"))

	serverDir := filepath.Join(tmpDir, "servers", "legacy")
	if err := os.MkdirAll(serverDir, 0755); err != nil {
		t.Fatalf("MkdirAll failed: %v", err)
	}

	legacyConfig := `name = "legacy"
version = "1.21.4"
loader = "vanilla"
memory = "2G"
port = 25565

[rcon]
enabled = true
password = "secret"
port = 25575

[world]
name = "world"
seed = ""
gamemode = "survival"
difficulty = "normal"
max_players = 20
motd = "A Minecraft Server"

[backup]
enabled = false
schedule = "0 */6 * * *"
type = "world"

[backup.retention]
keep_last = 10
keep_days = 7
min_backups = 3
`

	configPath := filepath.Join(serverDir, "hestia.toml")
	if err := os.WriteFile(configPath, []byte(legacyConfig), 0644); err != nil {
		t.Fatalf("WriteFile failed: %v", err)
	}

	cfg, err := store.LoadConfig("legacy")
	if err != nil {
		t.Fatalf("LoadConfig failed: %v", err)
	}

	if cfg.ConfigVersion != CurrentConfigVersion {
		t.Fatalf("expected config version %d, got %d", CurrentConfigVersion, cfg.ConfigVersion)
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatalf("ReadFile failed: %v", err)
	}

	content := string(data)
	if !strings.Contains(content, "config_version = 2") {
		t.Fatalf("expected migrated config_version in file, got:\n%s", content)
	}
	if strings.Contains(content, "\ntype = ") {
		t.Fatalf("expected legacy backup.type to be removed, got:\n%s", content)
	}
}

func TestJarPath(t *testing.T) {
	tmpDir := t.TempDir()
	store := NewStore(filepath.Join(tmpDir, "servers"))

	path := store.JarPath("myserver")
	expected := filepath.Join(tmpDir, "servers", "myserver", "data", "server.jar")
	if path != expected {
		t.Errorf("expected %s, got %s", expected, path)
	}
}
