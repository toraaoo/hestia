package server

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newRmCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "rm <name>",
		Aliases: []string{"remove", "delete"},
		Short:   "Remove a server",
		Args:    cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return withClient(cmd, func(c *client.Client) error {
				if err := c.DeleteServer(cmd.Context(), args[0]); err != nil {
					return err
				}
				fmt.Printf("Removed server %s\n", args[0])
				return nil
			})
		},
	}
}
