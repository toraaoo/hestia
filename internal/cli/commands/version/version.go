package version

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/version"
)

func NewCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "version",
		Short: "Show hestia version",
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Println(version.Info())
		},
	}
}
