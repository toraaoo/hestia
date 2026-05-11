package mod

import (
	"fmt"

	"github.com/spf13/cobra"
)

func newInstallCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "install <server> <mod>",
		Short: "Install a mod to a server",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			return fmt.Errorf("not implemented")
		},
	}
}
