package server

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newConsoleCmd() *cobra.Command {
	var useRCON bool
	var lines int

	cmd := &cobra.Command{
		Use:        "console <name>",
		Short:      "Send commands to server (DEPRECATED: use 'attach')",
		Args:       cobra.ExactArgs(1),
		Deprecated: "use 'hestia server attach' instead",
		RunE: func(cmd *cobra.Command, args []string) error {
			fmt.Fprintln(os.Stderr, "Warning: 'console' is deprecated, use 'attach' instead")
			return withClient(cmd, func(c *client.Client) error {
				return runAttach(cmd.Context(), c, args[0], useRCON, lines)
			})
		},
	}

	cmd.Flags().BoolVar(&useRCON, "rcon", false, "Use RCON for commands (shows responses)")
	cmd.Flags().IntVarP(&lines, "lines", "n", 100, "Number of log lines to show initially")
	return cmd
}
