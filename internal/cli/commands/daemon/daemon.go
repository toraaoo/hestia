package daemon

import (
	"context"
	"fmt"
	"os/exec"
	"time"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

func NewCmd() *cobra.Command {
	cmd := &cobra.Command{Use: "daemon", Short: "Manage the hestia daemon"}
	cmd.AddCommand(newStartCmd(), newStopCmd(), newStatusCmd())
	return cmd
}

func newClient() *client.Client {
	return client.New(config.DefaultSockPath())
}

func isDaemonRunning(ctx context.Context, c *client.Client) bool {
	return c.Do(ctx, "GET", "/ping", nil, nil) == nil
}

func newStartCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "start",
		Short: "Start the daemon",
		RunE: func(cmd *cobra.Command, args []string) error {
			ctx := cmd.Context()
			c := newClient()

			if isDaemonRunning(ctx, c) {
				fmt.Fprintln(cmd.OutOrStdout(), "daemon already running")
				return nil
			}

			daemonCmd := exec.Command("hestiad")
			daemonCmd.Stdout = nil
			daemonCmd.Stderr = nil
			daemonCmd.Stdin = nil
			if err := daemonCmd.Start(); err != nil {
				return fmt.Errorf("failed to start daemon: %w", err)
			}

			deadline := time.Now().Add(2 * time.Second)
			for time.Now().Before(deadline) {
				if isDaemonRunning(ctx, c) {
					fmt.Fprintln(cmd.OutOrStdout(), "daemon started")
					return nil
				}
				time.Sleep(100 * time.Millisecond)
			}

			return fmt.Errorf("daemon failed to start within timeout")
		},
	}
}

func newStopCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "stop",
		Short: "Stop the daemon",
		RunE: func(cmd *cobra.Command, args []string) error {
			ctx := cmd.Context()
			c := newClient()

			if !isDaemonRunning(ctx, c) {
				fmt.Fprintln(cmd.OutOrStdout(), "daemon not running")
				return nil
			}

			if err := c.Do(ctx, "POST", "/shutdown", nil, nil); err != nil {
				return fmt.Errorf("failed to send shutdown: %w", err)
			}

			deadline := time.Now().Add(5 * time.Second)
			for time.Now().Before(deadline) {
				if !isDaemonRunning(ctx, c) {
					fmt.Fprintln(cmd.OutOrStdout(), "daemon stopped")
					return nil
				}
				time.Sleep(100 * time.Millisecond)
			}

			return fmt.Errorf("daemon did not stop within timeout")
		},
	}
}

func newStatusCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "status",
		Short: "Show daemon status",
		RunE: func(cmd *cobra.Command, args []string) error {
			c := newClient()
			if isDaemonRunning(cmd.Context(), c) {
				fmt.Fprintln(cmd.OutOrStdout(), "daemon: running")
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "daemon: not running")
			}
			return nil
		},
	}
}
