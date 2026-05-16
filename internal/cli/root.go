package cli

import (
	"context"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/app"
	cmdconfig "github.com/toraaoo/hestia/internal/cli/commands/config"
	cmddaemon "github.com/toraaoo/hestia/internal/cli/commands/daemon"
	cmdmod "github.com/toraaoo/hestia/internal/cli/commands/mod"
	cmdserver "github.com/toraaoo/hestia/internal/cli/commands/server"
	cmdversion "github.com/toraaoo/hestia/internal/cli/commands/version"
	cmdversions "github.com/toraaoo/hestia/internal/cli/commands/versions"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/jar"
)

type App struct {
	Config    *config.Config
	Client    *client.Client
	Providers *jar.Registry
}

func NewApp(ctx context.Context) (*App, error) {
	cliApp, err := app.NewCLIApp(ctx)
	if err != nil {
		return nil, err
	}
	return &App{
		Config:    cliApp.Config,
		Client:    cliApp.Client,
		Providers: cliApp.Providers,
	}, nil
}

func (a *App) RootCommand() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "hestia",
		Short: "Hestia - Minecraft server manager",
	}
	serverCommands := cmdserver.NewCommands(a.Client, a.Providers)
	cmd.AddCommand(
		serverCommands.CreateCmd(),
		serverCommands.PsCmd(),
		serverCommands.StartCmd(),
		serverCommands.StopCmd(),
		serverCommands.RestartCmd(),
		serverCommands.RmCmd(),
		serverCommands.LogsCmd(),
		serverCommands.AttachCmd(),
		serverCommands.InspectCmd(),
		serverCommands.UpgradeCmd(),
		serverCommands.ConfigureCmd(),
		serverCommands.BackupCmd(),
		cmdversion.NewCmd(),
		cmdmod.NewCmd(),
		cmddaemon.NewCmd(a.Client),
		cmdconfig.NewCmd(),
		cmdversions.NewCmd(a.Client, a.Providers),
	)
	return cmd
}

func Execute() error {
	app, err := NewApp(context.Background())
	if err != nil {
		return err
	}
	return app.RootCommand().Execute()
}
