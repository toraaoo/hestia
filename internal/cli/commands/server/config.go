package server

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strconv"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

func newConfigCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "config <name> [key] [value]",
		Short: "View or modify server config",
		Args:  cobra.RangeArgs(1, 3),
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			c := client.New(cfg.Daemon.Sock)
			name := args[0]

			if len(args) == 1 {
				var resp map[string]any
				if err := c.Do(context.Background(), "GET", "/servers/"+name+"/config", nil, &resp); err != nil {
					return err
				}
				enc := json.NewEncoder(os.Stdout)
				enc.SetIndent("", "  ")
				return enc.Encode(resp)
			}

			if len(args) == 2 {
				var resp map[string]any
				if err := c.Do(context.Background(), "GET", "/servers/"+name+"/config", nil, &resp); err != nil {
					return err
				}
				key := args[1]
				if val, ok := resp[key]; ok {
					fmt.Println(val)
				} else {
					return fmt.Errorf("unknown config key: %s", key)
				}
				return nil
			}

			key, value := args[1], args[2]
			updates := make(map[string]any)

			switch key {
			case "memory":
				updates["memory"] = value
			case "port":
				p, err := strconv.Atoi(value)
				if err != nil {
					return fmt.Errorf("invalid port: %s", value)
				}
				updates["port"] = p
			case "world.gamemode", "world.difficulty", "world.motd":
				subkey := key[6:]
				updates["world"] = map[string]any{subkey: value}
			case "world.max_players":
				n, err := strconv.Atoi(value)
				if err != nil {
					return fmt.Errorf("invalid number: %s", value)
				}
				updates["world"] = map[string]any{"max_players": n}
			default:
				return fmt.Errorf("cannot set key: %s", key)
			}

			body, _ := json.Marshal(updates)
			if err := c.Do(context.Background(), "PUT", "/servers/"+name+"/config", bytes.NewReader(body), nil); err != nil {
				return err
			}
			fmt.Printf("Updated %s\n", key)
			return nil
		},
	}
	return cmd
}
