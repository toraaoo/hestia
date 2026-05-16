package server

import (
	"encoding/json"
	"fmt"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newBackupCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "backup",
		Short: "Manage server backups",
	}
	cmd.AddCommand(
		sc.newBackupCreateCmd(),
		sc.newBackupListCmd(),
		sc.newBackupRestoreCmd(),
		sc.newBackupDeleteCmd(),
		sc.newBackupPruneCmd(),
	)
	return cmd
}

func (sc *Commands) newBackupCreateCmd() *cobra.Command {
	var (
		full    bool
		force   bool
		jsonOut bool
	)

	cmd := &cobra.Command{
		Use:   "create <server>",
		Short: "Create a backup",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				req := client.BackupRequest{Force: force}
				if full {
					req.Type = "full"
				} else {
					req.Type = "world"
				}

				info, err := c.CreateBackup(cmd.Context(), args[0], req)
				if err != nil {
					return err
				}

				if jsonOut {
					enc := json.NewEncoder(cmd.OutOrStdout())
					enc.SetIndent("", "  ")
					return enc.Encode(info)
				}

				_, err = fmt.Fprintf(cmd.OutOrStdout(), "Created backup: %s (%s)\n", info.Name, formatBytes(info.Size))
				return err
			})
		},
	}

	cmd.Flags().BoolVar(&full, "full", false, "Full backup (world + config + plugins)")
	cmd.Flags().BoolVar(&force, "force", false, "Force backup even without RCON (unsafe)")
	cmd.Flags().BoolVar(&jsonOut, "json", false, "Output as JSON")
	return cmd
}

func (sc *Commands) newBackupListCmd() *cobra.Command {
	var jsonOut bool

	cmd := &cobra.Command{
		Use:   "list <server>",
		Short: "List backups",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				backups, err := c.ListBackups(cmd.Context(), args[0])
				if err != nil {
					return err
				}

				if jsonOut {
					enc := json.NewEncoder(cmd.OutOrStdout())
					enc.SetIndent("", "  ")
					return enc.Encode(backups)
				}

				if len(backups) == 0 {
					_, err = fmt.Fprintln(cmd.OutOrStdout(), "No backups found")
					return err
				}

				headers := []string{"NAME", "TYPE", "SIZE", "CREATED"}
				widths := []int{36, 8, 10, 20}
				var rows [][]string

				for _, b := range backups {
					rows = append(rows, []string{
						b.Name,
						b.Type,
						formatBytes(b.Size),
						b.CreatedAt.Format("2006-01-02 15:04:05"),
					})
				}

				_, err = fmt.Fprint(cmd.OutOrStdout(), ui.RenderTable(headers, rows, widths))
				return err
			})
		},
	}

	cmd.Flags().BoolVar(&jsonOut, "json", false, "Output as JSON")
	return cmd
}

func (sc *Commands) newBackupRestoreCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "restore <server> <backup>",
		Short: "Restore from backup",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				result, err := c.RestoreBackup(cmd.Context(), args[0], args[1])
				if err != nil {
					return err
				}

				if _, err := fmt.Fprintf(cmd.OutOrStdout(), "Restored: %s\n", result["restored"]); err != nil {
					return err
				}
				if wasRunning, ok := result["was_running"].(bool); ok && wasRunning {
					_, err := fmt.Fprintln(cmd.OutOrStdout(), "Server was stopped for restore. Restart with: hestia server start", args[0])
					return err
				}
				return nil
			})
		},
	}
	return cmd
}

func (sc *Commands) newBackupDeleteCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "delete <server> <backup>",
		Short: "Delete a backup",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				if err := c.DeleteBackup(cmd.Context(), args[0], args[1]); err != nil {
					return err
				}
				_, err := fmt.Fprintf(cmd.OutOrStdout(), "Deleted: %s\n", args[1])
				return err
			})
		},
	}
	return cmd
}

func (sc *Commands) newBackupPruneCmd() *cobra.Command {
	var (
		keepLast   int
		keepDays   int
		minBackups int
	)

	cmd := &cobra.Command{
		Use:   "prune <server>",
		Short: "Remove old backups",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
				req := client.PruneRequest{
					KeepLast:   keepLast,
					KeepDays:   keepDays,
					MinBackups: minBackups,
				}

				result, err := c.PruneBackups(cmd.Context(), args[0], req)
				if err != nil {
					return err
				}

				if result.Deleted == 0 {
					_, err := fmt.Fprintln(cmd.OutOrStdout(), "No backups to prune")
					return err
				}

				if _, err := fmt.Fprintf(cmd.OutOrStdout(), "Deleted %d backup(s):\n", result.Deleted); err != nil {
					return err
				}
				for _, name := range result.Names {
					if _, err := fmt.Fprintf(cmd.OutOrStdout(), "  - %s\n", name); err != nil {
						return err
					}
				}
				return nil
			})
		},
	}

	cmd.Flags().IntVar(&keepLast, "keep-last", 0, "Keep N most recent backups")
	cmd.Flags().IntVar(&keepDays, "keep-days", 0, "Keep backups newer than N days")
	cmd.Flags().IntVar(&minBackups, "min-backups", 0, "Always keep at least N backups")
	return cmd
}

func formatBytes(b int64) string {
	const unit = 1024
	if b < unit {
		return fmt.Sprintf("%d B", b)
	}
	div, exp := int64(unit), 0
	for n := b / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(b)/float64(div), "KMGTPE"[exp])
}
