package jar

import "fmt"

var providers = map[string]func() JarProvider{}

func Register(name string, factory func() JarProvider) {
	providers[name] = factory
}

func GetProvider(name string) (JarProvider, error) {
	f, ok := providers[name]
	if !ok {
		return nil, fmt.Errorf("unknown jar provider: %s", name)
	}
	return f(), nil
}

func ListProviders() []string {
	names := make([]string, 0, len(providers))
	for name := range providers {
		names = append(names, name)
	}
	return names
}

func init() {
	Register("vanilla", func() JarProvider { return VanillaProvider{} })
}
