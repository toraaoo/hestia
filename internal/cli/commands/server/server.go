package server

import (
	"github.com/spf13/cobra"
)

func NewCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "server",
		Short: "Manage Minecraft servers",
	}
	cmd.AddCommand(
		newCreateCmd(),
		newLsCmd(),
		newInspectCmd(),
		newRmCmd(),
		newStartCmd(),
		newStopCmd(),
		newRestartCmd(),
		newLogsCmd(),
		newConsoleCmd(),
		newConfigCmd(),
	)
	return cmd
}
