package cli

import (
	"github.com/spf13/cobra"
	cmdconfig "github.com/toraaoo/hestia/internal/cli/commands/config"
	cmddaemon "github.com/toraaoo/hestia/internal/cli/commands/daemon"
	cmdserver "github.com/toraaoo/hestia/internal/cli/commands/server"
	cmdversions "github.com/toraaoo/hestia/internal/cli/commands/versions"
)

func newRootCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "hestia",
		Short: "Hestia — Minecraft server manager",
		//SilenceUsage: true,
		//SilenceErrors: true,
	}
	cmd.AddCommand(
		cmddaemon.NewCmd(),
		cmdconfig.NewCmd(),
		cmdversions.NewCmd(),
		cmdserver.NewCmd(),
	)
	return cmd
}

func Execute() error {
	return newRootCmd().Execute()
}
