package server

import (
	"bufio"
	"fmt"
	"net/http"
	"strings"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newLogsCmd() *cobra.Command {
	var follow bool
	var lines int

	cmd := &cobra.Command{
		Use:   "logs <name>",
		Short: "Show server logs",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			if follow {
				return withClient(cmd, func(c *client.Client) error {
					return streamLogs(cmd, c, args[0], lines)
				})
			}

			return withClient(cmd, func(c *client.Client) error {
				logs, err := c.GetLogs(cmd.Context(), args[0], lines)
				if err != nil {
					return err
				}
				for _, l := range logs {
					fmt.Print(l.Text)
				}
				return nil
			})
		},
	}

	cmd.Flags().BoolVarP(&follow, "follow", "f", false, "Follow log output")
	cmd.Flags().IntVarP(&lines, "lines", "n", 100, "Number of lines to show")
	return cmd
}

func streamLogs(cmd *cobra.Command, c *client.Client, name string, lines int) error {
	path := fmt.Sprintf("/servers/%s/logs?follow=true&lines=%d", name, lines)
	req, err := http.NewRequestWithContext(cmd.Context(), "GET", "http://hestiad"+path, nil)
	if err != nil {
		return err
	}

	resp, err := c.DoRaw(cmd.Context(), req)
	if err != nil {
		return err
	}
	defer func() { _ = resp.Body.Close() }()

	scanner := bufio.NewScanner(resp.Body)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "data: ") {
			fmt.Println(strings.TrimPrefix(line, "data: "))
		}
	}
	return scanner.Err()
}
