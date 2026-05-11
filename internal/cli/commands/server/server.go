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
		newPsCmd(),
		newInspectCmd(),
		newRmCmd(),
		newStartCmd(),
		newStopCmd(),
		newRestartCmd(),
		newUpgradeCmd(),
		newLogsCmd(),
		newAttachCmd(),
		newConsoleCmd(),
		newConfigCmd(),
		newBackupCmd(),
	)
	return cmd
}
