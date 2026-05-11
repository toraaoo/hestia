package mod

import "github.com/spf13/cobra"

func NewCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "mod",
		Short: "Manage server mods and plugins",
	}
	cmd.AddCommand(
		newInstallCmd(),
		newListCmd(),
		newRemoveCmd(),
	)
	return cmd
}
