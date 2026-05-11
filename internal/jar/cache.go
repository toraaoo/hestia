package jar

import (
	"encoding/json"
	"os"
	"path/filepath"
	"time"

	"github.com/toraaoo/hestia/internal/config"
)

const cacheTTL = time.Hour

type cachedManifest struct {
	Manifest  *VersionManifest `json:"manifest"`
	FetchedAt time.Time        `json:"fetchedAt"`
}

func cacheDir() string {
	return filepath.Join(config.DefaultDir(), "cache")
}

func manifestCachePath() string {
	return filepath.Join(cacheDir(), "versions.json")
}

func loadCachedManifest() (*VersionManifest, bool) {
	data, err := os.ReadFile(manifestCachePath())
	if err != nil {
		return nil, false
	}
	var cached cachedManifest
	if err := json.Unmarshal(data, &cached); err != nil {
		return nil, false
	}
	if time.Since(cached.FetchedAt) > cacheTTL {
		return nil, false
	}
	return cached.Manifest, true
}

func saveManifestCache(m *VersionManifest) error {
	if err := os.MkdirAll(cacheDir(), 0755); err != nil {
		return err
	}
	cached := cachedManifest{
		Manifest:  m,
		FetchedAt: time.Now(),
	}
	data, err := json.Marshal(cached)
	if err != nil {
		return err
	}
	return os.WriteFile(manifestCachePath(), data, 0644)
}
