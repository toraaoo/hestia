package server

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func newStopCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "stop <name>",
		Short: "Stop a server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return withClient(cmd, func(c *client.Client) error {
				name := args[0]
				spinner := ui.NewSpinner(os.Stdout, fmt.Sprintf("Stopping %s...", name))
				spinner.Start()
				err := c.StopServer(cmd.Context(), name)
				spinner.Stop()
				if err != nil {
					return err
				}
				fmt.Printf("%s Stopped server %s\n", ui.StateStopped.Render("✓"), name)
				return nil
			})
		},
	}
}
