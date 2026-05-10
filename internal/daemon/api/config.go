package api

import (
	"encoding/json"
	"net/http"

	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/server"
)

func handleGetConfig(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/config")

	cfg, err := server.LoadConfig(name)
	if err != nil {
		http.Error(w, "server not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(cfg)
}

func handleUpdateConfig(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/config")

	existing, err := server.LoadConfig(name)
	if err != nil {
		http.Error(w, "server not found", http.StatusNotFound)
		return
	}

	if proc := procManager.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		http.Error(w, "stop server before changing config", http.StatusConflict)
		return
	}

	var updates map[string]any
	if err := json.NewDecoder(r.Body).Decode(&updates); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if v, ok := updates["memory"].(string); ok {
		existing.Memory = v
	}
	if v, ok := updates["port"].(float64); ok {
		existing.Port = int(v)
	}
	if world, ok := updates["world"].(map[string]any); ok {
		if v, ok := world["gamemode"].(string); ok {
			existing.World.Gamemode = v
		}
		if v, ok := world["difficulty"].(string); ok {
			existing.World.Difficulty = v
		}
		if v, ok := world["max_players"].(float64); ok {
			existing.World.MaxPlayers = int(v)
		}
		if v, ok := world["motd"].(string); ok {
			existing.World.MOTD = v
		}
	}

	if err := existing.Save(); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if err := existing.WriteProperties(); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(existing)
}
