package jar

import (
	"fmt"
	"sort"
)

type LoaderFactory func() Loader

type Registry struct {
	loaders map[string]LoaderFactory
}

func NewRegistry() *Registry {
	return &Registry{loaders: make(map[string]LoaderFactory)}
}

func (r *Registry) Register(name string, factory LoaderFactory) {
	r.loaders[name] = factory
}

func (r *Registry) GetLoader(name string) (Loader, error) {
	f, ok := r.loaders[name]
	if !ok {
		return nil, fmt.Errorf("unknown loader: %s", name)
	}
	return f(), nil
}

func (r *Registry) ListLoaders() []string {
	names := make([]string, 0, len(r.loaders))
	for name := range r.loaders {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}
