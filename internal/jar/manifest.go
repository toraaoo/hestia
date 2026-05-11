package jar

import (
	"encoding/json"
	"fmt"

	"github.com/toraaoo/hestia/internal/httpc"
)

const manifestURL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"

type VersionManifest struct {
	Latest struct {
		Release  string `json:"release"`
		Snapshot string `json:"snapshot"`
	} `json:"latest"`
	Versions []ManifestVersion `json:"versions"`
}

type ManifestVersion struct {
	ID          string `json:"id"`
	Type        string `json:"type"`
	URL         string `json:"url"`
	ReleaseTime string `json:"releaseTime"`
}

type VersionMeta struct {
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

func FetchManifest() (*VersionManifest, error) {
	if cached, ok := loadCachedManifest(); ok {
		return cached, nil
	}

	resp, err := httpc.Get(manifestURL)
	if err != nil {
		return nil, fmt.Errorf("fetch manifest: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("manifest api: %s", resp.Status)
	}

	var m VersionManifest
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		return nil, fmt.Errorf("decode manifest: %w", err)
	}

	_ = saveManifestCache(&m)
	return &m, nil
}

func FetchVersionMeta(url string) (*VersionMeta, error) {
	resp, err := httpc.Get(url)
	if err != nil {
		return nil, fmt.Errorf("fetch version meta: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("version meta api: %s", resp.Status)
	}

	var meta VersionMeta
	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		return nil, fmt.Errorf("decode version meta: %w", err)
	}
	return &meta, nil
}

func FindVersion(manifest *VersionManifest, versionID string) (*ManifestVersion, error) {
	for i := range manifest.Versions {
		if manifest.Versions[i].ID == versionID {
			return &manifest.Versions[i], nil
		}
	}
	return nil, fmt.Errorf("version not found: %s", versionID)
}

func GetLatestRelease() (string, error) {
	manifest, err := FetchManifest()
	if err != nil {
		return "", err
	}
	return manifest.Latest.Release, nil
}

func GetLatestSnapshot() (string, error) {
	manifest, err := FetchManifest()
	if err != nil {
		return "", err
	}
	return manifest.Latest.Snapshot, nil
}
