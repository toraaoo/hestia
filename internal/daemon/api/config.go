package api

import (
	"encoding/json"
	"net/http"

	"github.com/toraaoo/hestia/internal/daemon/process"
)

func (h *Handler) handleGetConfig(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	cfg, err := h.servers.LoadConfig(name)
	if err != nil {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(cfg)
}

func (h *Handler) handleUpdateConfig(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	existing, err := h.servers.LoadConfig(name)
	if err != nil {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	if proc := h.processes.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		writeError(w, "stop server before changing config", http.StatusConflict)
		return
	}

	var updates map[string]any
	if err := json.NewDecoder(r.Body).Decode(&updates); err != nil {
		writeError(w, err.Error(), http.StatusBadRequest)
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

	if err := h.servers.SaveConfig(existing); err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if err := h.servers.WriteProperties(existing); err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(existing)
}
