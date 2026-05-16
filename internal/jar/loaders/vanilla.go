package loaders

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/progress"
)

const minecraftVersionManifestURL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"

type VanillaProvider struct {
	http       *download.Client
	downloader *download.Client
}

func NewVanillaProvider(httpClient, downloadClient *download.Client) VanillaProvider {
	if httpClient == nil {
		httpClient = download.NewClient(nil, "hestia/1.0")
	}
	if downloadClient == nil {
		downloadClient = download.NewClient(nil, "hestia/1.0")
	}
	return VanillaProvider{http: httpClient, downloader: downloadClient}
}

func (VanillaProvider) Name() string { return "vanilla" }

type minecraftVersionManifest struct {
	Latest struct {
		Release  string `json:"release"`
		Snapshot string `json:"snapshot"`
	} `json:"latest"`
	Versions []minecraftManifestVersion `json:"versions"`
}

type minecraftManifestVersion struct {
	ID          string `json:"id"`
	Type        string `json:"type"`
	URL         string `json:"url"`
	ReleaseTime string `json:"releaseTime"`
}

type minecraftVersionMeta struct {
	Downloads struct {
		Server struct {
			URL  string `json:"url"`
			SHA1 string `json:"sha1"`
			Size int64  `json:"size"`
		} `json:"server"`
	} `json:"downloads"`
	JavaVersion struct {
		MajorVersion int `json:"majorVersion"`
	} `json:"javaVersion"`
}

func (p VanillaProvider) manifest() (*minecraftVersionManifest, error) {
	resp, err := p.http.Get(minecraftVersionManifestURL)
	if err != nil {
		return nil, fmt.Errorf("fetch Minecraft version manifest: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("minecraft version manifest api: %s", resp.Status)
	}

	var manifest minecraftVersionManifest
	if err := json.NewDecoder(resp.Body).Decode(&manifest); err != nil {
		return nil, fmt.Errorf("decode Minecraft version manifest: %w", err)
	}
	return &manifest, nil
}

func (p VanillaProvider) versionMeta(url string) (*minecraftVersionMeta, error) {
	resp, err := p.http.Get(url)
	if err != nil {
		return nil, fmt.Errorf("fetch Minecraft version meta: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("minecraft version meta api: %s", resp.Status)
	}

	var meta minecraftVersionMeta
	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		return nil, fmt.Errorf("decode Minecraft version meta: %w", err)
	}
	return &meta, nil
}

func (p VanillaProvider) findVersion(manifest *minecraftVersionManifest, versionID string) (*minecraftManifestVersion, error) {
	for i := range manifest.Versions {
		if manifest.Versions[i].ID == versionID {
			return &manifest.Versions[i], nil
		}
	}
	return nil, fmt.Errorf("version not found: %s", versionID)
}

func (p VanillaProvider) ListVersions(includeSnapshots bool) ([]jar.Version, error) {
	manifest, err := p.manifest()
	if err != nil {
		return nil, err
	}

	var versions []jar.Version
	for _, v := range manifest.Versions {
		if !includeSnapshots && v.Type != "release" {
			continue
		}
		versions = append(versions, jar.Version{
			ID:          v.ID,
			Type:        jar.VersionType(v.Type),
			ReleaseTime: v.ReleaseTime,
		})
	}
	return versions, nil
}

func (p VanillaProvider) DownloadServer(version, destPath string, cb progress.Callback) error {
	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryManifest, Message: "fetching manifest"})
	}

	manifest, err := p.manifest()
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	v, err := p.findVersion(manifest, version)
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	meta, err := p.versionMeta(v.URL)
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryManifest})
	}

	if meta.Downloads.Server.URL == "" {
		return fmt.Errorf("no server download for version %s", version)
	}

	if err := os.MkdirAll(filepath.Dir(destPath), 0755); err != nil {
		return fmt.Errorf("create dir: %w", err)
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryJar, Total: meta.Downloads.Server.Size})
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

	err = p.downloader.File(meta.Downloads.Server.URL, destPath, download.Options{
		SHA1:     meta.Downloads.Server.SHA1,
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

func (p VanillaProvider) GetJavaVersion(version string) (int, error) {
	manifest, err := p.manifest()
	if err != nil {
		return 0, err
	}

	v, err := p.findVersion(manifest, version)
	if err != nil {
		return 0, err
	}

	meta, err := p.versionMeta(v.URL)
	if err != nil {
		return 0, err
	}

	if meta.JavaVersion.MajorVersion == 0 {
		return 8, nil
	}
	return meta.JavaVersion.MajorVersion, nil
}

func (p VanillaProvider) LatestVersions() (release, snapshot string, err error) {
	manifest, err := p.manifest()
	if err != nil {
		return "", "", err
	}
	return manifest.Latest.Release, manifest.Latest.Snapshot, nil
}
