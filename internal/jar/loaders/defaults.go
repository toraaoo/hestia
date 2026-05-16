package loaders

import (
	"net/http"
	"time"

	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/jar"
)

func NewRegistry(httpClient, downloadClient *download.Client) *jar.Registry {
	if httpClient == nil {
		httpClient = download.NewClient(&http.Client{Timeout: 30 * time.Second}, "hestia/1.0")
	}
	if downloadClient == nil {
		downloadClient = download.NewClient(&http.Client{Timeout: 10 * time.Minute}, "hestia/1.0")
	}
	registry := jar.NewRegistry()
	registry.Register("vanilla", func() jar.Loader {
		return NewVanillaProvider(httpClient, downloadClient)
	})
	registry.Register("fabric", func() jar.Loader {
		return NewFabricLoader(httpClient, downloadClient)
	})
	return registry
}
