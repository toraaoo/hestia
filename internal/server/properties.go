package server

import (
	"fmt"
	"strings"
)

type property struct {
	key   string
	value string
}

func (c *Config) GenerateProperties() string {
	props := []property{
		{"server-port", fmt.Sprintf("%d", c.Port)},
		{"level-name", c.World.Name},
		{"gamemode", c.World.Gamemode},
		{"difficulty", c.World.Difficulty},
		{"max-players", fmt.Sprintf("%d", c.World.MaxPlayers)},
		{"motd", c.World.MOTD},
		{"enable-rcon", fmt.Sprintf("%t", c.RCON.Enabled)},
	}

	if c.World.Seed != "" {
		props = append(props, property{"level-seed", c.World.Seed})
	}

	if c.RCON.Enabled {
		props = append(props,
			property{"rcon.port", fmt.Sprintf("%d", c.RCON.Port)},
			property{"rcon.password", c.RCON.Password},
		)
	}

	var sb strings.Builder
	for _, p := range props {
		sb.WriteString(p.key)
		sb.WriteString("=")
		sb.WriteString(p.value)
		sb.WriteString("\n")
	}
	return sb.String()
}
