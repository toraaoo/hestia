package server

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

func newStartCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "start <name>",
		Short: "Start a server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			c := client.New(cfg.Daemon.Sock)
			if err := c.Do(context.Background(), "POST", "/servers/"+args[0]+"/start", nil, nil); err != nil {
				return err
			}

			fmt.Printf("Starting server %s\n", args[0])
			return nil
		},
	}
}
