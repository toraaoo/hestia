package server

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newConfigureCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "configure <name> [key] [value]",
		Short: "View or modify server configuration",
		Args:  cobra.RangeArgs(1, 3),
		RunE: func(cmd *cobra.Command, args []string) error {
			return withClient(cmd, func(c *client.Client) error {
				name := args[0]

				if len(args) == 1 {
					resp, err := c.GetConfig(cmd.Context(), name)
					if err != nil {
						return err
					}
					enc := json.NewEncoder(os.Stdout)
					enc.SetIndent("", "  ")
					return enc.Encode(resp)
				}

				if len(args) == 2 {
					resp, err := c.GetConfig(cmd.Context(), name)
					if err != nil {
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

				if err := c.UpdateConfig(cmd.Context(), name, updates); err != nil {
					return err
				}
				fmt.Printf("Updated %s\n", key)
				return nil
			})
		},
	}
	return cmd
}
