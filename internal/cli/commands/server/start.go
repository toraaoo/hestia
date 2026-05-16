package server

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newStartCmd() *cobra.Command {
	var attach bool
	var useRCON bool
	var lines int

	cmd := &cobra.Command{
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

				if !attach {
					return nil
				}

				if err := waitForRunning(cmd.Context(), c, name); err != nil {
					return fmt.Errorf("wait for server: %w", err)
				}
				return runAttach(cmd.Context(), c, name, useRCON, lines)
			})
		},
	}

	cmd.Flags().BoolVarP(&attach, "attach", "a", false, "Attach after starting (stream logs + send commands)")
	cmd.Flags().BoolVarP(&useRCON, "rcon", "r", false, "Use RCON for commands when attaching (shows responses)")
	cmd.Flags().IntVarP(&lines, "lines", "n", 100, "Number of log lines to show initially when attaching")
	return cmd
}
