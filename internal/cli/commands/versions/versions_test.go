package versions

import (
	"os"
	"testing"

	"github.com/toraaoo/hestia/internal/jar"
)

func TestPrintVersions_DoesNotPanicOnMissingReleaseTime(t *testing.T) {
	// Fabric versions don't include releaseTime; ensure formatting is safe.
	versions := []jar.Version{
		{ID: "1.20.1", ReleaseTime: ""},
		{ID: "1.20", ReleaseTime: "2023-06"},
		{ID: "1.19.4", ReleaseTime: "2023-03-14T00:00:00+00:00"},
	}

	// Avoid polluting test output.
	tmp, err := os.CreateTemp(t.TempDir(), "stdout-*")
	if err != nil {
		t.Fatalf("CreateTemp: %v", err)
	}
	old := os.Stdout
	os.Stdout = tmp
	t.Cleanup(func() {
		os.Stdout = old
		_ = tmp.Close()
	})

	defer func() {
		if r := recover(); r != nil {
			t.Fatalf("printVersions panicked: %v", r)
		}
	}()

	_ = printVersions(versions, struct {
		Release  string `json:"release"`
		Snapshot string `json:"snapshot"`
	}{}, false, false)
}
