package server

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newUpgradeCmd() *cobra.Command {
	var (
		version  string
		noBackup bool
		restart  bool
		force    bool
		jsonOut  bool
	)

	cmd := &cobra.Command{
		Use:   "upgrade <name> [version]",
		Short: "Upgrade server to a new version",
		Args:  cobra.RangeArgs(1, 2),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				name := args[0]

				targetVersion := version
				if len(args) > 1 {
					targetVersion = args[1]
				}
				if targetVersion == "" {
					latest, err := sc.latestVanillaRelease()
					if err != nil {
						return fmt.Errorf("get latest version: %w", err)
					}
					targetVersion = latest
				}

				info, err := c.GetServer(cmd.Context(), name)
				if err != nil {
					return err
				}
				currentVersion, _ := info["version"].(string)

				if currentVersion == targetVersion {
					fmt.Printf("Server %s already at version %s\n", name, targetVersion)
					return nil
				}

				if !force && isDowngrade(currentVersion, targetVersion) {
					fmt.Printf("Warning: downgrading from %s to %s\n", currentVersion, targetVersion)
					fmt.Print("Continue? [y/N] ")
					reader := bufio.NewReader(os.Stdin)
					resp, _ := reader.ReadString('\n')
					resp = strings.TrimSpace(strings.ToLower(resp))
					if resp != "y" && resp != "yes" {
						fmt.Println("Aborted")
						return nil
					}
				}

				req := client.UpgradeRequest{
					Version:  targetVersion,
					NoBackup: noBackup,
				}

				if jsonOut {
					result, err := c.UpgradeServer(cmd.Context(), name, req)
					if err != nil {
						return err
					}
					enc := json.NewEncoder(os.Stdout)
					enc.SetIndent("", "  ")
					return enc.Encode(result)
				}

				fmt.Printf("Upgrading %s: %s → %s\n", name, currentVersion, targetVersion)

				mp := ui.NewMultiProgress(os.Stdout)
				result, err := c.UpgradeServerWithProgress(cmd.Context(), name, req, mp.Handle)
				if err != nil {
					mp.Clear()
					return err
				}

				backupPath, _ := result["backup_path"].(string)
				if backupPath != "" {
					fmt.Printf("\nBackup: %s\n", backupPath)
				}
				fmt.Printf("Upgraded %s to %s\n", name, targetVersion)

				if restart {
					fmt.Printf("Starting %s...\n", name)
					if err := c.StartServer(cmd.Context(), name); err != nil {
						return fmt.Errorf("start server: %w", err)
					}
					fmt.Printf("Server %s started\n", name)
				}

				return nil
			})
		},
	}

	cmd.Flags().StringVar(&version, "version", "", "Target version (default: latest)")
	cmd.Flags().BoolVar(&noBackup, "no-backup", false, "Skip backup of current server.jar")
	cmd.Flags().BoolVar(&restart, "restart", false, "Restart server after upgrade")
	cmd.Flags().BoolVar(&force, "force", false, "Skip downgrade confirmation")
	cmd.Flags().BoolVar(&jsonOut, "json", false, "Output as JSON")

	return cmd
}

func isDowngrade(current, target string) bool {
	cParts := strings.Split(current, ".")
	tParts := strings.Split(target, ".")

	for i := 0; i < len(cParts) && i < len(tParts); i++ {
		var cNum, tNum int
		_, _ = fmt.Sscanf(cParts[i], "%d", &cNum)
		_, _ = fmt.Sscanf(tParts[i], "%d", &tNum)
		if tNum < cNum {
			return true
		}
		if tNum > cNum {
			return false
		}
	}
	return len(tParts) < len(cParts)
}
