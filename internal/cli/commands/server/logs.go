package server

import (
	"bufio"
	"context"
	"fmt"
	"net/http"
	"strings"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
)

type logLine struct {
	Text string `json:"text"`
}

func newLogsCmd() *cobra.Command {
	var follow bool
	var lines int

	cmd := &cobra.Command{
		Use:   "logs <name>",
		Short: "Show server logs",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			if follow {
				return streamLogs(cfg.Daemon.Sock, args[0], lines)
			}

			c := client.New(cfg.Daemon.Sock)
			path := fmt.Sprintf("/servers/%s/logs?lines=%d", args[0], lines)
			var logs []logLine
			if err := c.Do(context.Background(), "GET", path, nil, &logs); err != nil {
				return err
			}

			for _, l := range logs {
				fmt.Print(l.Text)
			}
			return nil
		},
	}

	cmd.Flags().BoolVarP(&follow, "follow", "f", false, "Follow log output")
	cmd.Flags().IntVarP(&lines, "lines", "n", 100, "Number of lines to show")
	return cmd
}

func streamLogs(sock, name string, lines int) error {
	c := client.New(sock)
	path := fmt.Sprintf("/servers/%s/logs?follow=true&lines=%d", name, lines)

	req, err := http.NewRequest("GET", "http://hestiad"+path, nil)
	if err != nil {
		return err
	}

	resp, err := c.DoRaw(context.Background(), req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	scanner := bufio.NewScanner(resp.Body)
	for scanner.Scan() {
		line := scanner.Text()
		if strings.HasPrefix(line, "data: ") {
			fmt.Println(strings.TrimPrefix(line, "data: "))
		}
	}
	return scanner.Err()
}
