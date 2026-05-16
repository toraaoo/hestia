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

	provider, err := jarRegistry.GetProvider(jarName)
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
	resp.Latest.Release, resp.Latest.Snapshot, err = jarRegistry.ResolveLatestVersions(provider)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(resp)
}
