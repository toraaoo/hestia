package jar

type VersionType string

const (
	VersionTypeRelease  VersionType = "release"
	VersionTypeSnapshot VersionType = "snapshot"
)

type Version struct {
	ID          string      `json:"id"`
	Type        VersionType `json:"type"`
	ReleaseTime string      `json:"releaseTime"`
	JavaVersion int         `json:"javaVersion,omitempty"`
}

type JarProvider interface {
	Name() string
	ListVersions(includeSnapshots bool) ([]Version, error)
	DownloadServer(version, destPath string) error
	GetJavaVersion(version string) (int, error)
}
