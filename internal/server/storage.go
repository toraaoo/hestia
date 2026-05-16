package server

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/BurntSushi/toml"
)

type Store struct {
	serversDir string
}

func NewStore(serversDir string) *Store {
	return &Store{serversDir: serversDir}
}

func (s *Store) Create(name, version string) (*Config, error) {
	dir := s.ServerDir(name)
	if _, err := os.Stat(dir); err == nil {
		return nil, fmt.Errorf("server %q already exists", name)
	}

	cfg := DefaultConfig(name, version)
	if err := cfg.ResolvePorts(); err != nil {
		return nil, err
	}

	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("create server dir: %w", err)
	}

	if err := os.MkdirAll(s.DataDir(name), 0755); err != nil {
		_ = os.RemoveAll(dir)
		return nil, fmt.Errorf("create data dir: %w", err)
	}

	if err := s.SaveConfig(cfg); err != nil {
		_ = os.RemoveAll(dir)
		return nil, err
	}

	if err := s.WriteProperties(cfg); err != nil {
		_ = os.RemoveAll(dir)
		return nil, err
	}

	return cfg, nil
}

func (s *Store) Delete(name string) error {
	dir := s.ServerDir(name)
	if _, err := os.Stat(dir); os.IsNotExist(err) {
		return fmt.Errorf("server %q not found", name)
	}
	return os.RemoveAll(dir)
}

func (s *Store) Exists(name string) bool {
	_, err := os.Stat(s.ServerDir(name))
	return err == nil
}

func (s *Store) List() ([]string, error) {
	entries, err := os.ReadDir(s.serversDir)
	if os.IsNotExist(err) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	var names []string
	for _, e := range entries {
		if e.IsDir() {
			configPath := filepath.Join(s.serversDir, e.Name(), "hestia.toml")
			if _, err := os.Stat(configPath); err == nil {
				names = append(names, e.Name())
			}
		}
	}
	sort.Strings(names)
	return names, nil
}

func (s *Store) LoadConfig(name string) (*Config, error) {
	path := filepath.Join(s.ServerDir(name), "hestia.toml")
	var cfg Config
	if _, err := toml.DecodeFile(path, &cfg); err != nil {
		return nil, fmt.Errorf("load config: %w", err)
	}
	return &cfg, nil
}

func (s *Store) SaveConfig(cfg *Config) error {
	dir := s.ServerDir(cfg.Name)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create server dir: %w", err)
	}
	path := filepath.Join(dir, "hestia.toml")
	f, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create config: %w", err)
	}
	defer func() { _ = f.Close() }()
	return toml.NewEncoder(f).Encode(cfg)
}

func (s *Store) WriteProperties(cfg *Config) error {
	dir := s.DataDir(cfg.Name)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create data dir: %w", err)
	}
	path := filepath.Join(dir, "server.properties")
	return os.WriteFile(path, []byte(cfg.GenerateProperties()), 0644)
}

func (s *Store) ServerDir(name string) string {
	return filepath.Join(s.serversDir, name)
}

func (s *Store) ServersDir() string {
	return s.serversDir
}

func (s *Store) DataDir(name string) string {
	return filepath.Join(s.ServerDir(name), "data")
}

func (s *Store) JarPath(name string) string {
	return filepath.Join(s.DataDir(name), "server.jar")
}

func (s *Store) BackupsDir(name string) string {
	return filepath.Join(s.ServerDir(name), "backups")
}

type BackupInfo struct {
	Path    string
	Version string
	Time    time.Time
}

func (s *Store) BackupJar(name string) (string, error) {
	cfg, err := s.LoadConfig(name)
	if err != nil {
		return "", err
	}

	jarPath := s.JarPath(name)
	if _, err := os.Stat(jarPath); os.IsNotExist(err) {
		return "", fmt.Errorf("server.jar not found")
	}

	backupDir := s.BackupsDir(name)
	if err := os.MkdirAll(backupDir, 0755); err != nil {
		return "", fmt.Errorf("create backups dir: %w", err)
	}

	timestamp := time.Now().Format("20060102-150405")
	backupName := fmt.Sprintf("server-%s-%s.jar", cfg.Version, timestamp)
	backupPath := filepath.Join(backupDir, backupName)

	src, err := os.Open(jarPath)
	if err != nil {
		return "", fmt.Errorf("open jar: %w", err)
	}
	defer func() { _ = src.Close() }()

	dst, err := os.Create(backupPath)
	if err != nil {
		return "", fmt.Errorf("create backup: %w", err)
	}
	defer func() { _ = dst.Close() }()

	if _, err := io.Copy(dst, src); err != nil {
		return "", fmt.Errorf("copy jar: %w", err)
	}

	return backupPath, nil
}

func (s *Store) ListJarBackups(name string) ([]BackupInfo, error) {
	backupDir := s.BackupsDir(name)
	entries, err := os.ReadDir(backupDir)
	if os.IsNotExist(err) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	var backups []BackupInfo
	for _, e := range entries {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".jar") {
			continue
		}
		info, err := e.Info()
		if err != nil {
			continue
		}
		backups = append(backups, BackupInfo{
			Path:    filepath.Join(backupDir, e.Name()),
			Version: extractVersion(e.Name()),
			Time:    info.ModTime(),
		})
	}

	sort.Slice(backups, func(i, j int) bool {
		return backups[i].Time.After(backups[j].Time)
	})

	return backups, nil
}

func extractVersion(filename string) string {
	name := strings.TrimSuffix(filename, ".jar")
	name = strings.TrimPrefix(name, "server-")
	parts := strings.Split(name, "-")
	if len(parts) >= 1 {
		return parts[0]
	}
	return ""
}

func (s *Store) PruneJarBackups(name string, keep int) error {
	backups, err := s.ListJarBackups(name)
	if err != nil {
		return err
	}
	if len(backups) <= keep {
		return nil
	}
	for _, b := range backups[keep:] {
		_ = os.Remove(b.Path)
	}
	return nil
}
