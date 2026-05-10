package server

import (
	"context"
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

func newRmCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "rm <name>",
		Aliases: []string{"remove", "delete"},
		Short:   "Remove a server",
		Args:    cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			c := client.New(cfg.Daemon.Sock)
			if err := c.Do(context.Background(), "DELETE", "/servers/"+args[0], nil, nil); err != nil {
				return err
			}

			fmt.Printf("Removed server %s\n", args[0])
			return nil
		},
	}
}
