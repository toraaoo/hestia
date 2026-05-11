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

// IsDaemonRunning checks if daemon is responding to ping.
func IsDaemonRunning(ctx context.Context, c *client.Client) bool {
	return c.Do(ctx, "GET", "/ping", nil, nil) == nil
}

// StartDaemon spawns hestiad and waits for it to be ready.
func StartDaemon(ctx context.Context, c *client.Client) error {
	if IsDaemonRunning(ctx, c) {
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
		if IsDaemonRunning(ctx, c) {
			return nil
		}
		time.Sleep(100 * time.Millisecond)
	}

	return fmt.Errorf("daemon failed to start within timeout")
}

// EnsureDaemon starts daemon if not running.
func EnsureDaemon(ctx context.Context, c *client.Client) error {
	return StartDaemon(ctx, c)
}

func newStartCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "start",
		Short: "Start the daemon",
		RunE: func(cmd *cobra.Command, args []string) error {
			ctx := cmd.Context()
			c := newClient()

			if IsDaemonRunning(ctx, c) {
				_, _ = fmt.Fprintln(cmd.OutOrStdout(), "daemon already running")
				return nil
			}

			if err := StartDaemon(ctx, c); err != nil {
				return err
			}
			_, _ = fmt.Fprintln(cmd.OutOrStdout(), "daemon started")
			return nil
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

			if !IsDaemonRunning(ctx, c) {
				_, _ = fmt.Fprintln(cmd.OutOrStdout(), "daemon not running")
				return nil
			}

			if err := c.Do(ctx, "POST", "/shutdown", nil, nil); err != nil {
				return fmt.Errorf("failed to send shutdown: %w", err)
			}

			deadline := time.Now().Add(5 * time.Second)
			for time.Now().Before(deadline) {
				if !IsDaemonRunning(ctx, c) {
					_, _ = fmt.Fprintln(cmd.OutOrStdout(), "daemon stopped")
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
			if IsDaemonRunning(cmd.Context(), c) {
				_, _ = fmt.Fprintln(cmd.OutOrStdout(), "daemon: running")
			} else {
				_, _ = fmt.Fprintln(cmd.OutOrStdout(), "daemon: not running")
			}
			return nil
		},
	}
}
