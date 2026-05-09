package config

import (
	"os"
	"path/filepath"
	"runtime"

	"github.com/BurntSushi/toml"
)

type Config struct {
	Daemon DaemonConfig `toml:"daemon"`
}

type DaemonConfig struct {
	Sock     string `toml:"sock"`
	LogLevel string `toml:"log_level"`
}

func Load() (*Config, error) {
	cfg := &Config{
		Daemon: DaemonConfig{
			Sock:     DefaultSockPath(),
			LogLevel: "info",
		},
	}
	path := DefaultConfigPath()
	if _, err := os.Stat(path); os.IsNotExist(err) {
		return cfg, nil
	}
	_, err := toml.DecodeFile(path, cfg)
	return cfg, err
}

func DefaultDir() string {
	if dir := os.Getenv("HESTIA_DATA_DIR"); dir != "" {
		return dir
	}

	if runtime.GOOS == "windows" {
		if dir := os.Getenv("LOCALAPPDATA"); dir != "" {
			return filepath.Join(dir, "hestia")
		}
	}

	home, _ := os.UserHomeDir()
	return filepath.Join(home, ".hestia")
}

func DefaultConfigPath() string {
	return filepath.Join(DefaultDir(), "config.toml")
}

func DefaultSockPath() string {
	return filepath.Join(DefaultDir(), "daemon.sock")
}
