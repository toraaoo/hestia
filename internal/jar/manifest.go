package jar

import (
	"encoding/json"
	"fmt"

	"github.com/toraaoo/hestia/internal/httpc"
)

const manifestURL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"

type versionManifest struct {
	Latest struct {
		Release  string `json:"release"`
		Snapshot string `json:"snapshot"`
	} `json:"latest"`
	Versions []manifestVersion `json:"versions"`
}

type manifestVersion struct {
	ID          string `json:"id"`
	Type        string `json:"type"`
	URL         string `json:"url"`
	ReleaseTime string `json:"releaseTime"`
}

type versionMeta struct {
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

func fetchManifest() (*versionManifest, error) {
	if cached, ok := loadCachedManifest(); ok {
		return cached, nil
	}

	resp, err := httpc.Get(manifestURL)
	if err != nil {
		return nil, fmt.Errorf("fetch manifest: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("manifest api: %s", resp.Status)
	}

	var m versionManifest
	if err := json.NewDecoder(resp.Body).Decode(&m); err != nil {
		return nil, fmt.Errorf("decode manifest: %w", err)
	}

	saveManifestCache(&m)
	return &m, nil
}

func fetchVersionMeta(url string) (*versionMeta, error) {
	resp, err := httpc.Get(url)
	if err != nil {
		return nil, fmt.Errorf("fetch version meta: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("version meta api: %s", resp.Status)
	}

	var meta versionMeta
	if err := json.NewDecoder(resp.Body).Decode(&meta); err != nil {
		return nil, fmt.Errorf("decode version meta: %w", err)
	}
	return &meta, nil
}

func findVersion(manifest *versionManifest, versionID string) (*manifestVersion, error) {
	for i := range manifest.Versions {
		if manifest.Versions[i].ID == versionID {
			return &manifest.Versions[i], nil
		}
	}
	return nil, fmt.Errorf("version not found: %s", versionID)
}
