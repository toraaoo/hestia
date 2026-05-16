package server

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newCreateCmd() *cobra.Command {
	var (
		// Basic
		version string
		memory  string
		port    int
		loader  string

		// RCON
		rconEnabled  bool
		noRCON       bool
		rconPassword string
		rconPort     int

		// World
		worldName  string
		seed       string
		gamemode   string
		difficulty string
		maxPlayers int
		motd       string

		// Behavior
		detach  bool
		jsonOut bool
	)

	cmd := &cobra.Command{
		Use:   "create <name>",
		Short: "Create and start a new server",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				ver := version
				if ver == "" {
					latest, err := sc.latestVanillaRelease()
					if err != nil {
						return fmt.Errorf("get latest version: %w", err)
					}
					ver = latest
				}

				req := client.CreateRequest{
					Name:         args[0],
					Version:      ver,
					Memory:       memory,
					Port:         port,
					Loader:       loader,
					RCONPassword: rconPassword,
					RCONPort:     rconPort,
					WorldName:    worldName,
					Seed:         seed,
					Gamemode:     gamemode,
					Difficulty:   difficulty,
					MaxPlayers:   maxPlayers,
					MOTD:         motd,
				}

				// Handle --rcon / --no-rcon flags
				if noRCON {
					f := false
					req.RCONEnabled = &f
				} else if rconEnabled {
					t := true
					req.RCONEnabled = &t
				}

				if jsonOut {
					result, err := c.CreateServer(cmd.Context(), req)
					if err != nil {
						return err
					}
					enc := json.NewEncoder(os.Stdout)
					enc.SetIndent("", "  ")
					return enc.Encode(result)
				}

				mp := ui.NewMultiProgress(os.Stdout)
				_, err := c.CreateServerWithProgress(cmd.Context(), req, mp.Handle)
				if err != nil {
					mp.Clear()
					return err
				}
				fmt.Printf("\nCreated server %s (version %s)\n", args[0], ver)

				// Start server
				fmt.Printf("Starting %s...\n", args[0])
				if err := c.StartServer(cmd.Context(), args[0]); err != nil {
					return fmt.Errorf("start server: %w", err)
				}

				// Wait for server to be running
				if err := waitForRunning(cmd.Context(), c, args[0]); err != nil {
					return fmt.Errorf("wait for server: %w", err)
				}
				fmt.Printf("Server %s started\n", args[0])

				if detach {
					return nil
				}

				// Attach
				return runAttach(cmd.Context(), c, args[0], false, 100)
			})
		},
	}

	// Basic flags
	cmd.Flags().StringVarP(&version, "version", "v", "", "Minecraft version (default: latest)")
	cmd.Flags().StringVarP(&memory, "memory", "m", "", "Memory allocation (e.g. 2G)")
	cmd.Flags().IntVarP(&port, "port", "p", 0, "Server port (auto-assigned if 0)")
	cmd.Flags().StringVarP(&loader, "loader", "l", "", "Mod loader")
	_ = cmd.RegisterFlagCompletionFunc("loader", func(_ *cobra.Command, _ []string, _ string) ([]string, cobra.ShellCompDirective) {
		// Keep completion dynamic so adding a new loader only requires registry registration.
		names := sc.loaders.ListLoaders()
		for i, name := range names {
			// Include a description if available.
			if p, err := sc.loaders.GetLoader(name); err == nil {
				names[i] = name + "\t" + p.Name()
			}
		}
		return names, cobra.ShellCompDirectiveNoFileComp
	})

	// RCON flags
	cmd.Flags().BoolVarP(&rconEnabled, "rcon", "r", false, "Enable RCON")
	cmd.Flags().BoolVarP(&noRCON, "no-rcon", "N", false, "Disable RCON")
	cmd.Flags().StringVarP(&rconPassword, "rcon-password", "P", "", "RCON password")
	cmd.Flags().IntVarP(&rconPort, "rcon-port", "R", 0, "RCON port")

	// World flags
	cmd.Flags().StringVarP(&worldName, "world", "w", "", "World name")
	cmd.Flags().StringVarP(&seed, "seed", "s", "", "World seed")
	cmd.Flags().StringVarP(&gamemode, "gamemode", "g", "", "Gamemode: survival, creative, adventure, spectator")
	cmd.Flags().StringVarP(&difficulty, "difficulty", "D", "", "Difficulty: peaceful, easy, normal, hard")
	cmd.Flags().IntVarP(&maxPlayers, "max-players", "M", 0, "Maximum players")
	cmd.Flags().StringVarP(&motd, "motd", "t", "", "Server Message of the Day")

	// Behavior flags
	cmd.Flags().BoolVarP(&detach, "detach", "d", false, "Detach after starting (don't attach)")
	cmd.Flags().BoolVarP(&jsonOut, "json", "j", false, "Output as JSON (no progress)")

	return cmd
}

func (sc *Commands) latestVanillaRelease() (string, error) {
	provider, err := sc.loaders.GetLoader("vanilla")
	if err != nil {
		return "", err
	}
	release, _, err := sc.loaders.ResolveLatestVersions(provider)
	if err != nil {
		return "", err
	}
	return release, nil
}

func waitForRunning(ctx context.Context, c *client.Client, name string) error {
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		info, err := c.GetServer(ctx, name)
		if err != nil {
			return err
		}
		if state, ok := info["state"].(string); ok && state == "running" {
			return nil
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(100 * time.Millisecond):
		}
	}
	return fmt.Errorf("server did not start within timeout")
}
