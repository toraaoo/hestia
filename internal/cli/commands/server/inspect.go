package server

import (
	"encoding/json"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newInspectCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "inspect <name>",
		Short: "Show server details",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return withClient(cmd, func(c *client.Client) error {
				info, err := c.GetServer(cmd.Context(), args[0])
				if err != nil {
					return err
				}
				enc := json.NewEncoder(os.Stdout)
				enc.SetIndent("", "  ")
				return enc.Encode(info)
			})
		},
	}
}
