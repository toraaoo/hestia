package version

import (
	"fmt"
	"runtime/debug"
	"strings"
)

var (
	Version   = "dev"
	GitCommit = "unknown"
	BuildDate = "unknown"
)

func buildInfo() (commit, time string) {
	info, ok := debug.ReadBuildInfo()
	if !ok {
		return "", ""
	}
	for _, s := range info.Settings {
		switch s.Key {
		case "vcs.revision":
			commit = s.Value
		case "vcs.time":
			time = s.Value
		}
	}
	return commit, time
}

func resolve() (ver, commit, date string) {
	ver = strings.TrimSpace(Version)
	if ver == "" {
		ver = "dev"
	}

	commit = strings.TrimSpace(GitCommit)
	if commit == "" {
		commit = "unknown"
	}

	date = strings.TrimSpace(BuildDate)
	if date == "" {
		date = "unknown"
	}

	// Prefer build-time ldflags (GoReleaser / Makefile). For local `go build`, fall
	// back to Go's embedded VCS settings.
	if commit == "unknown" || date == "unknown" {
		biCommit, biTime := buildInfo()
		biCommit = strings.TrimSpace(biCommit)
		biTime = strings.TrimSpace(biTime)

		if commit == "unknown" && biCommit != "" {
			if len(biCommit) > 12 {
				biCommit = biCommit[:12]
			}
			commit = biCommit
		}
		if date == "unknown" && biTime != "" {
			date = biTime
		}
	}

	return ver, commit, date
}

func Info() string {
	ver, commit, date := resolve()
	return fmt.Sprintf("hestia %s\ncommit: %s\nbuilt: %s", ver, commit, date)
}
