package cli

import (
	"github.com/spf13/cobra"
	cmdconfig "github.com/toraaoo/hestia/internal/cli/commands/config"
	cmddaemon "github.com/toraaoo/hestia/internal/cli/commands/daemon"
	cmdmod "github.com/toraaoo/hestia/internal/cli/commands/mod"
	cmdserver "github.com/toraaoo/hestia/internal/cli/commands/server"
	cmdversion "github.com/toraaoo/hestia/internal/cli/commands/version"
	cmdversions "github.com/toraaoo/hestia/internal/cli/commands/versions"
)

func newRootCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "hestia",
		Short: "Hestia — Minecraft server manager",
	}
	cmd.AddCommand(
		cmdserver.NewCreateCmd(),
		cmdserver.NewPsCmd(),
		cmdserver.NewStartCmd(),
		cmdserver.NewStopCmd(),
		cmdserver.NewRestartCmd(),
		cmdserver.NewRmCmd(),
		cmdserver.NewLogsCmd(),
		cmdserver.NewAttachCmd(),
		cmdserver.NewInspectCmd(),
		cmdserver.NewUpgradeCmd(),
		cmdserver.NewConfigureCmd(),
		cmdserver.NewBackupCmd(),
		cmdversion.NewCmd(),
		cmdmod.NewCmd(),
		cmddaemon.NewCmd(),
		cmdconfig.NewCmd(),
		cmdversions.NewCmd(),
	)
	return cmd
}

func Execute() error {
	return newRootCmd().Execute()
}
