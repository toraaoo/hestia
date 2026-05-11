package version

import "fmt"

var (
	Version   = "dev"
	GitCommit = "unknown"
	BuildDate = "unknown"
)

func Info() string {
	return fmt.Sprintf("hestia %s\ncommit: %s\nbuilt: %s", Version, GitCommit, BuildDate)
}
