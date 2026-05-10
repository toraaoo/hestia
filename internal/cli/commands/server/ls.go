package server

import (
	"context"
	"encoding/json"
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

type serverInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Port    int    `json:"port"`
	State   string `json:"state"`
	PID     int    `json:"pid,omitempty"`
}

func newLsCmd() *cobra.Command {
	var jsonOut bool

	cmd := &cobra.Command{
		Use:     "ls",
		Aliases: []string{"list"},
		Short:   "List servers",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			c := client.New(cfg.Daemon.Sock)
			var servers []serverInfo
			if err := c.Do(context.Background(), "GET", "/servers", nil, &servers); err != nil {
				return err
			}

			if jsonOut {
				enc := json.NewEncoder(os.Stdout)
				enc.SetIndent("", "  ")
				return enc.Encode(servers)
			}

			if len(servers) == 0 {
				fmt.Println("No servers")
				return nil
			}

			fmt.Printf("%-20s %-12s %-8s %-10s %s\n", "NAME", "VERSION", "PORT", "STATE", "PID")
			for _, s := range servers {
				pid := ""
				if s.PID > 0 {
					pid = fmt.Sprintf("%d", s.PID)
				}
				fmt.Printf("%-20s %-12s %-8d %-10s %s\n", s.Name, s.Version, s.Port, s.State, pid)
			}
			return nil
		},
	}

	cmd.Flags().BoolVar(&jsonOut, "json", false, "Output as JSON")
	return cmd
}
