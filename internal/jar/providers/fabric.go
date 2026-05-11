package providers

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"path/filepath"

	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/httpc"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/progress"
)

func init() {
	jar.Register("fabric", func() jar.JarProvider { return FabricProvider{} })
}

type FabricProvider struct{}

func (FabricProvider) Name() string { return "fabric" }

const fabricAPIBase = "https://meta.fabricmc.net/v2"

type fabricGameVersion struct {
	Version string `json:"version"`
	Stable  bool   `json:"stable"`
}

type fabricLoaderVersion struct {
	Version string `json:"version"`
	Stable  bool   `json:"stable"`
}

type fabricInstallerVersion struct {
	Version string `json:"version"`
	Stable  bool   `json:"stable"`
}

func fabricGet[T any](url string) (T, error) {
	var zero T
	resp, err := httpc.Get(url)
	if err != nil {
		return zero, fmt.Errorf("fabric api: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != http.StatusOK {
		return zero, fmt.Errorf("fabric api: %s", resp.Status)
	}

	var result T
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return zero, fmt.Errorf("decode fabric response: %w", err)
	}
	return result, nil
}

func (FabricProvider) ListVersions(includeSnapshots bool) ([]jar.Version, error) {
	gameVersions, err := fabricGet[[]fabricGameVersion](fabricAPIBase + "/versions/game")
	if err != nil {
		return nil, err
	}

	versions := make([]jar.Version, 0, len(gameVersions))
	for _, v := range gameVersions {
		if !includeSnapshots && !v.Stable {
			continue
		}
		vType := jar.VersionTypeRelease
		if !v.Stable {
			vType = jar.VersionTypeSnapshot
		}
		versions = append(versions, jar.Version{
			ID:   v.Version,
			Type: vType,
		})
	}
	return versions, nil
}

func fetchFabricLatestLoader() (string, error) {
	loaders, err := fabricGet[[]fabricLoaderVersion](fabricAPIBase + "/versions/loader")
	if err != nil {
		return "", err
	}
	for _, l := range loaders {
		if l.Stable {
			return l.Version, nil
		}
	}
	if len(loaders) > 0 {
		return loaders[0].Version, nil
	}
	return "", fmt.Errorf("no fabric loader versions available")
}

func fetchFabricLatestInstaller() (string, error) {
	installers, err := fabricGet[[]fabricInstallerVersion](fabricAPIBase + "/versions/installer")
	if err != nil {
		return "", err
	}
	for _, i := range installers {
		if i.Stable {
			return i.Version, nil
		}
	}
	if len(installers) > 0 {
		return installers[0].Version, nil
	}
	return "", fmt.Errorf("no fabric installer versions available")
}

func (FabricProvider) DownloadServer(version, destPath string, cb progress.Callback) error {
	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryManifest, Message: "fetching fabric metadata"})
	}

	loader, err := fetchFabricLatestLoader()
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	installer, err := fetchFabricLatestInstaller()
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryManifest})
	}

	url := fmt.Sprintf("%s/versions/loader/%s/%s/%s/server/jar", fabricAPIBase, version, loader, installer)

	if err := os.MkdirAll(filepath.Dir(destPath), 0755); err != nil {
		return fmt.Errorf("create dir: %w", err)
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryJar})
	}

	var progressFn func(downloaded, total int64)
	if cb != nil {
		progressFn = func(downloaded, total int64) {
			cb(progress.Event{
				Type:     progress.EventProgress,
				Category: progress.CategoryJar,
				Current:  downloaded,
				Total:    total,
			})
		}
	}

	err = download.File(url, destPath, download.Options{
		Progress: progressFn,
	})
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryJar, Error: err.Error()})
		}
		return err
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryJar})
	}
	return nil
}

func (FabricProvider) GetJavaVersion(version string) (int, error) {
	manifest, err := jar.FetchManifest()
	if err != nil {
		return 0, err
	}
	v, err := jar.FindVersion(manifest, version)
	if err != nil {
		return 0, err
	}
	meta, err := jar.FetchVersionMeta(v.URL)
	if err != nil {
		return 0, err
	}
	if meta.JavaVersion.MajorVersion == 0 {
		return 8, nil
	}
	return meta.JavaVersion.MajorVersion, nil
}
