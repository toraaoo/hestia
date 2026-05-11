package jar

import "fmt"

var providerRegistry = map[string]func() JarProvider{}

func Register(name string, factory func() JarProvider) {
	providerRegistry[name] = factory
}

func GetProvider(name string) (JarProvider, error) {
	f, ok := providerRegistry[name]
	if !ok {
		return nil, fmt.Errorf("unknown jar provider: %s", name)
	}
	return f(), nil
}

func ListProviders() []string {
	names := make([]string, 0, len(providerRegistry))
	for name := range providerRegistry {
		names = append(names, name)
	}
	return names
}
