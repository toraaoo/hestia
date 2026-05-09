package server

import (
	"fmt"
	"os"
	"path/filepath"
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
		os.RemoveAll(dir)
		return nil, err
	}

	if err := cfg.WriteProperties(); err != nil {
		os.RemoveAll(dir)
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
