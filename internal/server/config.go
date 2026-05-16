package server

import (
	"fmt"
	"net"

	"github.com/google/uuid"
)

type Config struct {
	Name    string `toml:"name" json:"name"`
	Version string `toml:"version" json:"version"`
	Loader  string `toml:"loader" json:"loader"`
	Memory  string `toml:"memory" json:"memory"`
	Port    int    `toml:"port" json:"port"`

	RCON   RCONConfig   `toml:"rcon" json:"rcon"`
	World  WorldConfig  `toml:"world" json:"world"`
	Backup BackupConfig `toml:"backup" json:"backup"`
}

type RCONConfig struct {
	Enabled  bool   `toml:"enabled" json:"enabled"`
	Password string `toml:"password" json:"password"`
	Port     int    `toml:"port" json:"port"`
}

type WorldConfig struct {
	Name       string `toml:"name" json:"name"`
	Seed       string `toml:"seed" json:"seed"`
	Gamemode   string `toml:"gamemode" json:"gamemode"`
	Difficulty string `toml:"difficulty" json:"difficulty"`
	MaxPlayers int    `toml:"max_players" json:"max_players"`
	MOTD       string `toml:"motd" json:"motd"`
}

type BackupConfig struct {
	Enabled   bool            `toml:"enabled" json:"enabled"`
	Schedule  string          `toml:"schedule" json:"schedule"`
	Type      string          `toml:"type" json:"type"`
	Retention RetentionConfig `toml:"retention" json:"retention"`
}

type RetentionConfig struct {
	KeepLast   int `toml:"keep_last" json:"keep_last"`
	KeepDays   int `toml:"keep_days" json:"keep_days"`
	MinBackups int `toml:"min_backups" json:"min_backups"`
}

func DefaultConfig(name, version string) *Config {
	return &Config{
		Name:    name,
		Version: version,
		Loader:  "vanilla",
		Memory:  "2G",
		Port:    0,
		RCON: RCONConfig{
			Enabled:  true,
			Password: uuid.New().String(),
			Port:     0,
		},
		World: WorldConfig{
			Name:       "world",
			Seed:       "",
			Gamemode:   "survival",
			Difficulty: "normal",
			MaxPlayers: 20,
			MOTD:       "A Minecraft Server",
		},
		Backup: BackupConfig{
			Enabled:  false,
			Schedule: "0 */6 * * *",
			Type:     "world",
			Retention: RetentionConfig{
				KeepLast:   10,
				KeepDays:   7,
				MinBackups: 3,
			},
		},
	}
}

func (c *Config) ResolvePorts() error {
	if c.Port == 0 {
		port, err := findFreePort(25565, 25600)
		if err != nil {
			return fmt.Errorf("resolve server port: %w", err)
		}
		c.Port = port
	}
	if c.RCON.Enabled && c.RCON.Port == 0 {
		port, err := findFreePort(25575, 25600)
		if err != nil {
			return fmt.Errorf("resolve rcon port: %w", err)
		}
		c.RCON.Port = port
	}
	return nil
}

func findFreePort(start, end int) (int, error) {
	for port := start; port <= end; port++ {
		if isPortFree(port) {
			return port, nil
		}
	}
	return 0, fmt.Errorf("no free port in range %d-%d", start, end)
}

func isPortFree(port int) bool {
	ln, err := net.Listen("tcp", fmt.Sprintf(":%d", port))
	if err != nil {
		return false
	}
	_ = ln.Close()
	return true
}
