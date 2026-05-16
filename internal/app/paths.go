package app

import (
	"path/filepath"

	"github.com/toraaoo/hestia/internal/config"
)

type Paths struct {
	DataDir    string
	ConfigPath string
	SockPath   string
	ServersDir string
	JREDir     string
	CacheDir   string
}

func ResolvePaths() (Paths, error) {
	dataDir := config.DefaultDir()
	return Paths{
		DataDir:    dataDir,
		ConfigPath: filepath.Join(dataDir, "config.toml"),
		SockPath:   filepath.Join(dataDir, "daemon.sock"),
		ServersDir: filepath.Join(dataDir, "servers"),
		JREDir:     filepath.Join(dataDir, "jre"),
		CacheDir:   filepath.Join(dataDir, "cache"),
	}, nil
}

func LoadConfig(paths Paths) (*config.Config, error) {
	return config.LoadFile(paths.ConfigPath, config.Config{
		Daemon: config.DaemonConfig{
			Sock:     paths.SockPath,
			LogLevel: "info",
		},
	})
}
