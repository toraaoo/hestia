package server

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

func Create(name, version string) (*Config, error) {
	dir := ServerDir(name)
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

	if err := cfg.Save(); err != nil {
		_ = os.RemoveAll(dir)
		return nil, err
	}

	if err := cfg.WriteProperties(); err != nil {
		_ = os.RemoveAll(dir)
		return nil, err
	}

	return cfg, nil
}

func Delete(name string) error {
	dir := ServerDir(name)
	if _, err := os.Stat(dir); os.IsNotExist(err) {
		return fmt.Errorf("server %q not found", name)
	}
	return os.RemoveAll(dir)
}

func Exists(name string) bool {
	dir := ServerDir(name)
	_, err := os.Stat(dir)
	return err == nil
}

func List() ([]string, error) {
	dir := ServersDir()
	entries, err := os.ReadDir(dir)
	if os.IsNotExist(err) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	var names []string
	for _, e := range entries {
		if e.IsDir() {
			configPath := filepath.Join(dir, e.Name(), "hestia.toml")
			if _, err := os.Stat(configPath); err == nil {
				names = append(names, e.Name())
			}
		}
	}
	return names, nil
}

func JarPath(name string) string {
	return filepath.Join(ServerDir(name), "server.jar")
}

func BackupsDir(name string) string {
	return filepath.Join(ServerDir(name), "backups")
}

type BackupInfo struct {
	Path    string
	Version string
	Time    time.Time
}

func BackupJar(name string) (string, error) {
	cfg, err := LoadConfig(name)
	if err != nil {
		return "", err
	}

	jarPath := JarPath(name)
	if _, err := os.Stat(jarPath); os.IsNotExist(err) {
		return "", fmt.Errorf("server.jar not found")
	}

	backupDir := BackupsDir(name)
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

func ListBackups(name string) ([]BackupInfo, error) {
	backupDir := BackupsDir(name)
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
		version := extractVersion(e.Name())
		backups = append(backups, BackupInfo{
			Path:    filepath.Join(backupDir, e.Name()),
			Version: version,
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

func PruneBackups(name string, keep int) error {
	backups, err := ListBackups(name)
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
