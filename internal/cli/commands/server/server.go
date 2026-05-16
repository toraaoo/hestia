package server

import (
	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/jar"
)

type Commands struct {
	client    *client.Client
	providers *jar.Registry
}

func NewCommands(client *client.Client, providers *jar.Registry) *Commands {
	return &Commands{client: client, providers: providers}
}

func (c *Commands) CreateCmd() *cobra.Command    { return c.newCreateCmd() }
func (c *Commands) PsCmd() *cobra.Command        { return c.newPsCmd() }
func (c *Commands) InspectCmd() *cobra.Command   { return c.newInspectCmd() }
func (c *Commands) RmCmd() *cobra.Command        { return c.newRmCmd() }
func (c *Commands) StartCmd() *cobra.Command     { return c.newStartCmd() }
func (c *Commands) StopCmd() *cobra.Command      { return c.newStopCmd() }
func (c *Commands) RestartCmd() *cobra.Command   { return c.newRestartCmd() }
func (c *Commands) UpgradeCmd() *cobra.Command   { return c.newUpgradeCmd() }
func (c *Commands) LogsCmd() *cobra.Command      { return c.newLogsCmd() }
func (c *Commands) AttachCmd() *cobra.Command    { return c.newAttachCmd() }
func (c *Commands) ConfigureCmd() *cobra.Command { return c.newConfigureCmd() }
func (c *Commands) BackupCmd() *cobra.Command    { return c.newBackupCmd() }
