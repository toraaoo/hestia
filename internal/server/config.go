package server

import (
	"fmt"
	"net"
	"os"
	"path/filepath"

	"github.com/BurntSushi/toml"
	"github.com/google/uuid"
	"github.com/toraaoo/hestia/internal/config"
)

type Config struct {
	Name    string `toml:"name" json:"name"`
	Version string `toml:"version" json:"version"`
	Jar     string `toml:"jar" json:"jar"`
	Memory  string `toml:"memory" json:"memory"`
	Port    int    `toml:"port" json:"port"`

	RCON  RCONConfig  `toml:"rcon" json:"rcon"`
	World WorldConfig `toml:"world" json:"world"`
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

func DefaultConfig(name, version string) *Config {
	return &Config{
		Name:    name,
		Version: version,
		Jar:     "vanilla",
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

func LoadConfig(serverName string) (*Config, error) {
	path := filepath.Join(ServerDir(serverName), "hestia.toml")
	var cfg Config
	_, err := toml.DecodeFile(path, &cfg)
	if err != nil {
		return nil, fmt.Errorf("load config: %w", err)
	}
	return &cfg, nil
}

func (c *Config) Save() error {
	dir := ServerDir(c.Name)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("create server dir: %w", err)
	}
	path := filepath.Join(dir, "hestia.toml")
	f, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create config: %w", err)
	}
	defer func() { _ = f.Close() }()
	return toml.NewEncoder(f).Encode(c)
}

func ServersDir() string {
	return filepath.Join(config.DefaultDir(), "servers")
}

func ServerDir(name string) string {
	return filepath.Join(ServersDir(), name)
}
