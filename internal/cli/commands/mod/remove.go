package mod

import (
	"fmt"

	"github.com/spf13/cobra"
)

func newRemoveCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "remove <server> <mod>",
		Short: "Remove a mod from a server",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			return fmt.Errorf("not implemented")
		},
	}
}
