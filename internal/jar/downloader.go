package jar

import (
	"crypto/sha1"
	"encoding/hex"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
)

type VanillaProvider struct{}

func (VanillaProvider) Name() string { return "vanilla" }

func (VanillaProvider) ListVersions(includeSnapshots bool) ([]Version, error) {
	manifest, err := fetchManifest()
	if err != nil {
		return nil, err
	}

	var versions []Version
	for _, v := range manifest.Versions {
		if !includeSnapshots && v.Type != "release" {
			continue
		}
		versions = append(versions, Version{
			ID:          v.ID,
			Type:        VersionType(v.Type),
			ReleaseTime: v.ReleaseTime,
		})
	}
	return versions, nil
}

func (VanillaProvider) DownloadServer(version, destPath string) error {
	manifest, err := fetchManifest()
	if err != nil {
		return err
	}

	v, err := findVersion(manifest, version)
	if err != nil {
		return err
	}

	meta, err := fetchVersionMeta(v.URL)
	if err != nil {
		return err
	}

	if meta.Downloads.Server.URL == "" {
		return fmt.Errorf("no server download for version %s", version)
	}

	if err := os.MkdirAll(filepath.Dir(destPath), 0755); err != nil {
		return fmt.Errorf("create dir: %w", err)
	}

	resp, err := http.Get(meta.Downloads.Server.URL)
	if err != nil {
		return fmt.Errorf("download server: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("server download: %s", resp.Status)
	}

	tmpPath := destPath + ".tmp"
	f, err := os.Create(tmpPath)
	if err != nil {
		return fmt.Errorf("create file: %w", err)
	}

	h := sha1.New()
	if _, err := io.Copy(io.MultiWriter(f, h), resp.Body); err != nil {
		f.Close()
		os.Remove(tmpPath)
		return fmt.Errorf("write file: %w", err)
	}
	f.Close()

	got := hex.EncodeToString(h.Sum(nil))
	if got != meta.Downloads.Server.SHA1 {
		os.Remove(tmpPath)
		return fmt.Errorf("sha1 mismatch: got %s, want %s", got, meta.Downloads.Server.SHA1)
	}

	return os.Rename(tmpPath, destPath)
}

func (VanillaProvider) GetJavaVersion(version string) (int, error) {
	manifest, err := fetchManifest()
	if err != nil {
		return 0, err
	}

	v, err := findVersion(manifest, version)
	if err != nil {
		return 0, err
	}

	meta, err := fetchVersionMeta(v.URL)
	if err != nil {
		return 0, err
	}

	if meta.JavaVersion.MajorVersion == 0 {
		return 8, nil
	}
	return meta.JavaVersion.MajorVersion, nil
}

func GetLatestRelease() (string, error) {
	manifest, err := fetchManifest()
	if err != nil {
		return "", err
	}
	return manifest.Latest.Release, nil
}

func GetLatestSnapshot() (string, error) {
	manifest, err := fetchManifest()
	if err != nil {
		return "", err
	}
	return manifest.Latest.Snapshot, nil
}
