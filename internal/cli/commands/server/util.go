package server

import (
	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/commands/daemon"
	"github.com/toraaoo/hestia/internal/client"
)

func (c *Commands) withClient(cmd *cobra.Command, fn func(*client.Client) error) error {
	client := c.client
	if err := daemon.EnsureDaemon(cmd.Context(), client); err != nil {
		return err
	}
	return fn(client)
}
