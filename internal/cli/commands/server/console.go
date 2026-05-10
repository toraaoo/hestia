package server

import (
	"bufio"
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
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
				return withClient(cmd, func(c *client.Client) error {
					return stdinConsole(cmd, c, args[0])
				})
			}
			if err := rconConsole(args[0]); err != nil {
				fmt.Fprintf(os.Stderr, "RCON unavailable (%v), using stdin\n", err)
				return withClient(cmd, func(c *client.Client) error {
					return stdinConsole(cmd, c, args[0])
				})
			}
			return nil
		},
	}

	cmd.Flags().BoolVar(&useStdin, "stdin", false, "Use stdin instead of RCON (no response display)")
	return cmd
}

func stdinConsole(cmd *cobra.Command, c *client.Client, name string) error {
	scanner := bufio.NewScanner(os.Stdin)
	ctx := cmd.Context()

	fmt.Printf("Console for %s (Ctrl+D to exit)\n> ", name)
	for scanner.Scan() {
		command := scanner.Text()
		if command == "" {
			fmt.Print("> ")
			continue
		}
		if err := c.SendConsoleCommand(ctx, name, command); err != nil {
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
