package loaders

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"path/filepath"

	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/httpc"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/progress"
)

const fabricAPIBase = "https://meta.fabricmc.net/v2"

type FabricLoader struct{}

func NewFabricLoader() FabricLoader {
	return FabricLoader{}
}

func (FabricLoader) Name() string { return "fabric" }

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

type fabricCompatibleLoaderVersion struct {
	Loader       fabricLoaderVersion `json:"loader"`
	LauncherMeta struct {
		MinJavaVersion int `json:"min_java_version"`
	} `json:"launcherMeta"`
}

func (p FabricLoader) get(url string, target any) error {
	resp, err := httpc.Get(url)
	if err != nil {
		return fmt.Errorf("fabric api: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("fabric api: %s", resp.Status)
	}

	if err := json.NewDecoder(resp.Body).Decode(target); err != nil {
		return fmt.Errorf("decode fabric response: %w", err)
	}
	return nil
}

func (p FabricLoader) gameVersions() ([]fabricGameVersion, error) {
	var versions []fabricGameVersion
	if err := p.get(fabricAPIBase+"/versions/game", &versions); err != nil {
		return nil, err
	}
	return versions, nil
}

func (p FabricLoader) latestCompatibleLoader(gameVersion string) (fabricCompatibleLoaderVersion, error) {
	var versions []fabricCompatibleLoaderVersion
	if err := p.get(fabricAPIBase+"/versions/loader/"+url.PathEscape(gameVersion), &versions); err != nil {
		return fabricCompatibleLoaderVersion{}, err
	}
	for _, version := range versions {
		if version.Loader.Stable {
			return version, nil
		}
	}
	if len(versions) > 0 {
		return versions[0], nil
	}
	return fabricCompatibleLoaderVersion{}, fmt.Errorf("no fabric loader versions available for game version %s", gameVersion)
}

func (p FabricLoader) latestInstaller() (string, error) {
	var versions []fabricInstallerVersion
	if err := p.get(fabricAPIBase+"/versions/installer", &versions); err != nil {
		return "", err
	}
	for _, version := range versions {
		if version.Stable {
			return version.Version, nil
		}
	}
	if len(versions) > 0 {
		return versions[0].Version, nil
	}
	return "", fmt.Errorf("no fabric installer versions available")
}

func (p FabricLoader) ListVersions(includeSnapshots bool) ([]jar.Version, error) {
	gameVersions, err := p.gameVersions()
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

func (p FabricLoader) DownloadServer(version, destPath string, cb progress.Callback) error {
	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryManifest, Message: "fetching fabric metadata"})
	}

	loader, err := p.latestCompatibleLoader(version)
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	installer, err := p.latestInstaller()
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryManifest})
	}

	downloadURL := fmt.Sprintf(
		"%s/versions/loader/%s/%s/%s/server/jar",
		fabricAPIBase,
		url.PathEscape(version),
		url.PathEscape(loader.Loader.Version),
		url.PathEscape(installer),
	)

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

	err = download.File(downloadURL, destPath, download.Options{
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

func (p FabricLoader) GetJavaVersion(version string) (int, error) {
	loader, err := p.latestCompatibleLoader(version)
	if err != nil {
		return 0, err
	}
	if loader.LauncherMeta.MinJavaVersion == 0 {
		return 8, nil
	}
	return loader.LauncherMeta.MinJavaVersion, nil
}
