package jar

import "fmt"

type ProviderFactory func() Loader

type Registry struct {
	providers map[string]ProviderFactory
}

func NewRegistry() *Registry {
	return &Registry{providers: make(map[string]ProviderFactory)}
}

func (r *Registry) Register(name string, factory ProviderFactory) {
	r.providers[name] = factory
}

func (r *Registry) GetProvider(name string) (Loader, error) {
	f, ok := r.providers[name]
	if !ok {
		return nil, fmt.Errorf("unknown jar provider: %s", name)
	}
	return f(), nil
}

func (r *Registry) ListProviders() []string {
	names := make([]string, 0, len(r.providers))
	for name := range r.providers {
		names = append(names, name)
	}
	return names
}
