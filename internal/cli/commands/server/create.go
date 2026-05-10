package server

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

func newCreateCmd() *cobra.Command {
	var version, memory string
	var port int

	cmd := &cobra.Command{
		Use:   "create <name>",
		Short: "Create a new server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			req := map[string]any{
				"name":    args[0],
				"version": version,
			}
			if memory != "" {
				req["memory"] = memory
			}
			if port != 0 {
				req["port"] = port
			}

			body, _ := json.Marshal(req)
			c := client.New(cfg.Daemon.Sock)

			var resp map[string]any
			if err := c.Do(context.Background(), "POST", "/servers", bytes.NewReader(body), &resp); err != nil {
				return err
			}

			fmt.Printf("Created server %s (version %s)\n", args[0], version)
			return nil
		},
	}

	cmd.Flags().StringVar(&version, "version", "1.21.4", "Minecraft version")
	cmd.Flags().StringVar(&memory, "memory", "", "Memory allocation (e.g. 2G)")
	cmd.Flags().IntVar(&port, "port", 0, "Server port (auto-assigned if 0)")
	return cmd
}
