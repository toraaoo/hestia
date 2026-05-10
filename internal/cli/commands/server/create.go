package server

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newCreateCmd() *cobra.Command {
	var version, memory string
	var port int

	cmd := &cobra.Command{
		Use:   "create <name>",
		Short: "Create a new server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return withClient(cmd, func(c *client.Client) error {
				req := client.CreateRequest{
					Name:    args[0],
					Version: version,
					Memory:  memory,
					Port:    port,
				}
				if _, err := c.CreateServer(cmd.Context(), req); err != nil {
					return err
				}
				fmt.Printf("Created server %s (version %s)\n", args[0], version)
				return nil
			})
		},
	}

	cmd.Flags().StringVar(&version, "version", "1.21.4", "Minecraft version")
	cmd.Flags().StringVar(&memory, "memory", "", "Memory allocation (e.g. 2G)")
	cmd.Flags().IntVar(&port, "port", 0, "Server port (auto-assigned if 0)")
	return cmd
}
