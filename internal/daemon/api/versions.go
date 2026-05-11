package api

import (
	"encoding/json"
	"net/http"

	"github.com/toraaoo/hestia/internal/jar"
)

type versionsResponse struct {
	Latest struct {
		Release  string `json:"release"`
		Snapshot string `json:"snapshot"`
	} `json:"latest"`
	Versions []jar.Version `json:"versions"`
}

func handleVersions(w http.ResponseWriter, r *http.Request) {
	snapshots := r.URL.Query().Get("snapshots") == "true"
	jarName := r.URL.Query().Get("jar")
	if jarName == "" {
		jarName = "vanilla"
	}

	provider, err := jar.GetProvider(jarName)
	if err != nil {
		writeError(w, "unsupported jar type: "+jarName, http.StatusBadRequest)
		return
	}

	versions, err := provider.ListVersions(snapshots)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	resp := versionsResponse{Versions: versions}
	if jarName == "vanilla" {
		resp.Latest.Release, _ = jar.GetLatestRelease()
		resp.Latest.Snapshot, _ = jar.GetLatestSnapshot()
	} else {
		resp.Latest.Release, resp.Latest.Snapshot = jar.ComputeLatest(versions)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}
