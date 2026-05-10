package server

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/rcon"
	"github.com/toraaoo/hestia/internal/server"
)

func newConsoleCmd() *cobra.Command {
	var useStdin bool

	cmd := &cobra.Command{
		Use:   "console <name>",
		Short: "Send commands to server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			if useStdin {
				return stdinConsole(args[0])
			}
			if err := rconConsole(args[0]); err != nil {
				fmt.Fprintf(os.Stderr, "RCON unavailable (%v), using stdin\n", err)
				return stdinConsole(args[0])
			}
			return nil
		},
	}

	cmd.Flags().BoolVar(&useStdin, "stdin", false, "Use stdin instead of RCON (no response display)")
	return cmd
}

func stdinConsole(name string) error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}

	c := client.New(cfg.Daemon.Sock)
	scanner := bufio.NewScanner(os.Stdin)

	fmt.Printf("Console for %s (Ctrl+D to exit)\n> ", name)
	for scanner.Scan() {
		command := scanner.Text()
		if command == "" {
			fmt.Print("> ")
			continue
		}

		body, _ := json.Marshal(map[string]string{"command": command})
		if err := c.Do(context.Background(), "POST", "/servers/"+name+"/console", bytes.NewReader(body), nil); err != nil {
			fmt.Printf("Error: %v\n", err)
		}
		fmt.Print("> ")
	}
	fmt.Println()
	return nil
}

func rconConsole(name string) error {
	cfg, err := server.LoadConfig(name)
	if err != nil {
		return err
	}

	if !cfg.RCON.Enabled {
		return fmt.Errorf("RCON not enabled for server %s", name)
	}

	addr := fmt.Sprintf("localhost:%d", cfg.RCON.Port)
	rc, err := rcon.Dial(addr, cfg.RCON.Password)
	if err != nil {
		return err
	}
	defer rc.Close()

	scanner := bufio.NewScanner(os.Stdin)
	fmt.Printf("RCON console for %s (Ctrl+D to exit)\n> ", name)
	for scanner.Scan() {
		command := scanner.Text()
		if command == "" {
			fmt.Print("> ")
			continue
		}

		resp, err := rc.Execute(command)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
		} else if resp != "" {
			fmt.Println(resp)
		}
		fmt.Print("> ")
	}
	fmt.Println()
	return nil
}
