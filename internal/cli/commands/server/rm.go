package server

import (
	"fmt"
	"time"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newRmCmd() *cobra.Command {
	var force bool

	cmd := &cobra.Command{
		Use:     "rm <name>",
		Aliases: []string{"remove", "delete"},
		Short:   "Remove a server",
		Args:    cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				name := args[0]
				ctx := cmd.Context()

				if force {
					// Stop server first if running
					if err := c.StopServer(ctx, name); err == nil {
						// Wait for stop
						deadline := time.Now().Add(30 * time.Second)
						for time.Now().Before(deadline) {
							info, _ := c.GetServer(ctx, name)
							if state, ok := info["state"].(string); ok && state == "stopped" {
								break
							}
							time.Sleep(100 * time.Millisecond)
						}
					}
				}

				if err := c.DeleteServer(ctx, name); err != nil {
					return err
				}
				fmt.Printf("Removed server %s\n", name)
				return nil
			})
		},
	}

	cmd.Flags().BoolVarP(&force, "force", "f", false, "Stop server before removing")
	return cmd
}
