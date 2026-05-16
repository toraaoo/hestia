package server

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newStartCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "start <name>",
		Short: "Start a server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				name := args[0]
				spinner := ui.NewSpinner(os.Stdout, fmt.Sprintf("Starting %s...", name))
				spinner.Start()
				err := c.StartServer(cmd.Context(), name)
				spinner.Stop()
				if err != nil {
					return err
				}
				fmt.Printf("%s Started server %s\n", ui.StateRunning.Render("✓"), name)
				return nil
			})
		},
	}
}
