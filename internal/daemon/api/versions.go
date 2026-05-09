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

	provider := jar.VanillaProvider{}
	versions, err := provider.ListVersions(snapshots)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	resp := versionsResponse{Versions: versions}
	resp.Latest.Release, _ = jar.GetLatestRelease()
	resp.Latest.Snapshot, _ = jar.GetLatestSnapshot()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}
