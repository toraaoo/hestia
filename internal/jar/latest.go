package jar

// ComputeLatest derives "latest release" and "latest snapshot" from an already
// ordered versions list (newest first). This is used for providers that don't
// have a dedicated latest endpoint.
func ComputeLatest(versions []Version) (release, snapshot string) {
	for _, v := range versions {
		if release == "" && v.Type == VersionTypeRelease {
			release = v.ID
		}
		if snapshot == "" && v.Type == VersionTypeSnapshot {
			snapshot = v.ID
		}
		if release != "" && snapshot != "" {
			break
		}
	}
	return release, snapshot
}
