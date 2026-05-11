package jar

import "testing"

func TestComputeLatest(t *testing.T) {
	versions := []Version{
		{ID: "s2", Type: VersionTypeSnapshot},
		{ID: "r2", Type: VersionTypeRelease},
		{ID: "s1", Type: VersionTypeSnapshot},
		{ID: "r1", Type: VersionTypeRelease},
	}

	rel, snap := ComputeLatest(versions)
	if rel != "r2" {
		t.Fatalf("release=%q, want %q", rel, "r2")
	}
	if snap != "s2" {
		t.Fatalf("snapshot=%q, want %q", snap, "s2")
	}
}
