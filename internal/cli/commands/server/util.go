package server

import (
	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

func withClient(cmd *cobra.Command, fn func(*client.Client) error) error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}
	return fn(client.New(cfg.Daemon.Sock))
}
