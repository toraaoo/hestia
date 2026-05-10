package server

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/client"
)

func newLsCmd() *cobra.Command {
	var jsonOut bool

	cmd := &cobra.Command{
		Use:     "ls",
		Aliases: []string{"list"},
		Short:   "List servers",
		RunE: func(cmd *cobra.Command, args []string) error {
			return withClient(cmd, func(c *client.Client) error {
				servers, err := c.ListServers(cmd.Context())
				if err != nil {
					return err
				}

				if jsonOut {
					enc := json.NewEncoder(os.Stdout)
					enc.SetIndent("", "  ")
					return enc.Encode(servers)
				}

				if len(servers) == 0 {
					fmt.Println("No servers")
					return nil
				}

				fmt.Printf("%-20s %-12s %-8s %-10s %s\n", "NAME", "VERSION", "PORT", "STATE", "PID")
				for _, s := range servers {
					pid := ""
					if s.PID > 0 {
						pid = fmt.Sprintf("%d", s.PID)
					}
					fmt.Printf("%-20s %-12s %-8d %-10s %s\n", s.Name, s.Version, s.Port, s.State, pid)
				}
				return nil
			})
		},
	}

	cmd.Flags().BoolVar(&jsonOut, "json", false, "Output as JSON")
	return cmd
}
