package config

import (
	"fmt"

	"github.com/spf13/cobra"
)

func NewCmd() *cobra.Command {
	cmd := &cobra.Command{Use: "config", Short: "Manage hestia configuration"}
	cmd.AddCommand(newGetCmd(), newSetCmd())
	return cmd
}

func newGetCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "get <key>",
		Short: "Get a config value",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			// TODO: load config, print value at key
			fmt.Fprintf(cmd.OutOrStdout(), "%s: not implemented\n", args[0])
			return nil
		},
	}
}

func newSetCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "set <key> <value>",
		Short: "Set a config value",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			// TODO: load config, update key, write back
			fmt.Fprintf(cmd.OutOrStdout(), "set %s=%s: not implemented\n", args[0], args[1])
			return nil
		},
	}
}
