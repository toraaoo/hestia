package server

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
)

func (sc *Commands) newPsCmd() *cobra.Command {
	var jsonOut bool

	cmd := &cobra.Command{
		Use:     "ps",
		Aliases: []string{"ls", "list"},
		Short:   "Show server status",
		RunE: func(cmd *cobra.Command, _ []string) error {
			return sc.withClient(cmd, func(c *client.Client) error {
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

				rows := make([][]string, len(servers))
				for i, s := range servers {
					pid := ""
					if s.PID > 0 {
						pid = fmt.Sprintf("%d", s.PID)
					}
					status := ui.StateStyle(s.State).Render(s.State)
					rows[i] = []string{s.Name, s.Loader, s.Version, fmt.Sprintf("%d", s.Port), status, pid}
				}

				fmt.Println(ui.RenderTable(
					[]string{"NAME", "LOADER", "VERSION", "PORT", "STATUS", "PID"},
					rows,
					[]int{20, 10, 12, 8, 10, 8},
				))
				return nil
			})
		},
	}

	cmd.Flags().BoolVarP(&jsonOut, "json", "j", false, "Output as JSON")
	return cmd
}
