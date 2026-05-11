package versions

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/toraaoo/hestia/internal/cli/ui"
	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/jar"
	"golang.org/x/term"
)

func NewCmd() *cobra.Command {
	var snapshots, latest, jsonOut bool

	cmd := &cobra.Command{
		Use:   "versions",
		Short: "List available Minecraft versions",
		RunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load()
			if err != nil {
				return err
			}

			c := client.New(cfg.Daemon.Sock)
			path := "/versions"
			if snapshots {
				path += "?snapshots=true"
			}

			var resp VersionsResponse
			if err := c.Do(cmd.Context(), "GET", path, nil, &resp); err != nil {
				return fallbackLocal(snapshots, latest, jsonOut)
			}

			return printVersions(resp.Versions, resp.Latest, latest, jsonOut)
		},
	}

	cmd.Flags().BoolVar(&snapshots, "snapshots", false, "Include snapshot versions")
	cmd.Flags().BoolVar(&latest, "latest", false, "Show only latest versions")
	cmd.Flags().BoolVar(&jsonOut, "json", false, "Output as JSON")
	return cmd
}

type VersionsResponse struct {
	Latest struct {
		Release  string `json:"release"`
		Snapshot string `json:"snapshot"`
	} `json:"latest"`
	Versions []jar.Version `json:"versions"`
}

func fallbackLocal(snapshots, latest, jsonOut bool) error {
	provider, err := jar.GetProvider("vanilla")
	if err != nil {
		return err
	}
	versions, err := provider.ListVersions(snapshots)
	if err != nil {
		return err
	}

	latestRelease, _ := jar.GetLatestRelease()
	latestSnapshot, _ := jar.GetLatestSnapshot()

	resp := VersionsResponse{Versions: versions}
	resp.Latest.Release = latestRelease
	resp.Latest.Snapshot = latestSnapshot

	return printVersions(resp.Versions, resp.Latest, latest, jsonOut)
}

func printVersions(versions []jar.Version, latestInfo struct {
	Release  string `json:"release"`
	Snapshot string `json:"snapshot"`
}, latest, jsonOut bool) error {
	if latest {
		versions = filterLatest(versions, latestInfo.Release, latestInfo.Snapshot)
	}

	if jsonOut {
		enc := json.NewEncoder(os.Stdout)
		enc.SetIndent("", "  ")
		return enc.Encode(versions)
	}

	items := make([]string, len(versions))
	for i, v := range versions {
		marker := ""
		if v.ID == latestInfo.Release {
			marker = " (latest release)"
		} else if v.ID == latestInfo.Snapshot {
			marker = " (latest snapshot)"
		}
		items[i] = fmt.Sprintf("%s  %s  %s%s", v.ID, v.Type, v.ReleaseTime[:10], marker)
	}

	if !term.IsTerminal(int(os.Stdout.Fd())) || latest {
		for _, item := range items {
			fmt.Println(item)
		}
		return nil
	}

	return ui.RunPaginator(items, 20)
}

func filterLatest(versions []jar.Version, release, snapshot string) []jar.Version {
	var result []jar.Version
	for _, v := range versions {
		if v.ID == release || v.ID == snapshot {
			result = append(result, v)
		}
	}
	return result
}
