package loaders

import "github.com/toraaoo/hestia/internal/jar"

func NewRegistry() *jar.Registry {
	registry := jar.NewRegistry()
	registry.Register("vanilla", func() jar.Loader {
		return NewVanillaProvider()
	})
	registry.Register("fabric", func() jar.Loader {
		return NewFabricLoader()
	})
	return registry
}
