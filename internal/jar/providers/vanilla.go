package providers

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/progress"
)

func init() {
	jar.Register("vanilla", func() jar.JarProvider { return VanillaProvider{} })
}

type VanillaProvider struct{}

func (VanillaProvider) Name() string { return "vanilla" }

func (VanillaProvider) ListVersions(includeSnapshots bool) ([]jar.Version, error) {
	manifest, err := jar.FetchManifest()
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

func (VanillaProvider) DownloadServer(version, destPath string, cb progress.Callback) error {
	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryManifest, Message: "fetching manifest"})
	}

	manifest, err := jar.FetchManifest()
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	v, err := jar.FindVersion(manifest, version)
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryManifest, Error: err.Error()})
		}
		return err
	}

	meta, err := jar.FetchVersionMeta(v.URL)
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

	err = download.File(meta.Downloads.Server.URL, destPath, download.Options{
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

func (VanillaProvider) GetJavaVersion(version string) (int, error) {
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
