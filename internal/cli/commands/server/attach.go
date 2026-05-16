package server

import (
	"bufio"
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/rcon"
)

func (sc *Commands) newAttachCmd() *cobra.Command {
	var useRCON bool
	var lines int

	cmd := &cobra.Command{
		Use:   "attach <name>",
		Short: "Attach to server (stream logs + send commands)",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				return runAttach(cmd.Context(), c, args[0], useRCON, lines)
			})
		},
	}

	cmd.Flags().BoolVarP(&useRCON, "rcon", "r", false, "Use RCON for commands (shows responses)")
	cmd.Flags().IntVarP(&lines, "lines", "n", 100, "Number of log lines to show initially")
	return cmd
}

func runAttach(ctx context.Context, c *client.Client, name string, useRCON bool, lines int) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		fmt.Println("\nStopping server...")
		_ = c.StopServer(context.Background(), name)
		cancel()
	}()

	fmt.Printf("Attached to %s. Ctrl+C to stop, /detach to detach.\n", name)

	errCh := make(chan error, 2)

	go func() {
		errCh <- streamLogsAttach(ctx, c, name, lines)
	}()

	go func() {
		if useRCON {
			errCh <- readStdinRCON(ctx, c, name, cancel)
		} else {
			errCh <- readStdinHTTP(ctx, c, name, cancel)
		}
	}()

	err := <-errCh
	cancel()
	return err
}

func streamLogsAttach(ctx context.Context, c *client.Client, name string, lines int) error {
	path := fmt.Sprintf("/servers/%s/logs?follow=true&lines=%d", name, lines)
	req, err := http.NewRequestWithContext(ctx, "GET", "http://hestiad"+path, nil)
	if err != nil {
		return err
	}

	resp, err := c.DoRaw(ctx, req)
	if err != nil {
		return err
	}
	defer func() { _ = resp.Body.Close() }()

	scanner := bufio.NewScanner(resp.Body)
	for scanner.Scan() {
		select {
		case <-ctx.Done():
			return nil
		default:
			line := scanner.Text()
			if strings.HasPrefix(line, "data: ") {
				fmt.Println(strings.TrimPrefix(line, "data: "))
			}
		}
	}
	return scanner.Err()
}

func readStdinHTTP(ctx context.Context, c *client.Client, name string, cancel context.CancelFunc) error {
	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		select {
		case <-ctx.Done():
			return nil
		default:
			command := scanner.Text()
			if command == "/detach" {
				cancel()
				return nil
			}
			if command == "/stop" {
				fmt.Println("Stopping server...")
				_ = c.StopServer(context.Background(), name)
				cancel()
				return nil
			}
			if command != "" {
				if err := c.SendConsoleCommand(ctx, name, command); err != nil {
					fmt.Printf("Error: %v\n", err)
				}
			}
		}
	}
	return nil
}

func readStdinRCON(ctx context.Context, c *client.Client, name string, cancel context.CancelFunc) error {
	info, err := c.GetServerDetails(ctx, name)
	if err != nil {
		return fmt.Errorf("failed to get server config: %w", err)
	}

	if !info.RCON.Enabled {
		return fmt.Errorf("RCON not enabled for server %s", name)
	}

	addr := fmt.Sprintf("localhost:%d", info.RCON.Port)
	rc, err := rcon.Dial(addr, info.RCON.Password)
	if err != nil {
		return fmt.Errorf("failed to connect RCON: %w", err)
	}
	defer func() { _ = rc.Close() }()

	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		select {
		case <-ctx.Done():
			return nil
		default:
			command := scanner.Text()
			if command == "/detach" {
				cancel()
				return nil
			}
			if command == "/stop" {
				fmt.Println("Stopping server...")
				_ = c.StopServer(context.Background(), name)
				cancel()
				return nil
			}
			if command != "" {
				resp, err := rc.Execute(command)
				if err != nil {
					fmt.Printf("Error: %v\n", err)
				} else if resp != "" {
					fmt.Printf("> %s\n", resp)
				}
			}
		}
	}
	return nil
}
