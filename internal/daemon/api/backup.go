package api

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/toraaoo/hestia/internal/backup"
	"github.com/toraaoo/hestia/internal/daemon/process"
)

type createBackupRequest struct {
	Type  string `json:"type,omitempty"`
	Force bool   `json:"force,omitempty"`
}

type pruneRequest struct {
	KeepLast   int `json:"keep_last,omitempty"`
	KeepDays   int `json:"keep_days,omitempty"`
	MinBackups int `json:"min_backups,omitempty"`
}

type pruneResponse struct {
	Deleted int      `json:"deleted"`
	Names   []string `json:"names"`
}

func (h *Handler) handleCreateBackup(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	cfg, err := h.servers.LoadConfig(name)
	if err != nil {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	var req createBackupRequest
	if r.ContentLength > 0 {
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, err.Error(), http.StatusBadRequest)
			return
		}
	}

	backupType := backup.TypeWorld
	if req.Type == "full" {
		backupType = backup.TypeFull
	}

	opts := backup.Options{
		Type:       backupType,
		ServerName: name,
	}

	proc := h.processes.Get(name)
	isRunning := proc != nil && proc.GetState() == process.StateRunning

	if isRunning {
		if cfg.RCON.Enabled {
			opts.UseRCON = true
			opts.RCONAddr = fmt.Sprintf("localhost:%d", cfg.RCON.Port)
			opts.RCONPass = cfg.RCON.Password
		} else if !req.Force {
			writeError(w, "server running without RCON; use force=true for unsafe backup", http.StatusConflict)
			return
		}
	}

	info, err := h.backups.Create(opts)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if cfg.Backup.Enabled {
		policy := backup.RetentionPolicy{
			KeepLast:   cfg.Backup.Retention.KeepLast,
			KeepDays:   cfg.Backup.Retention.KeepDays,
			MinBackups: cfg.Backup.Retention.MinBackups,
		}
		_, _ = h.backups.Prune(name, policy)
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(info)
}

func (h *Handler) handleListBackups(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	if !h.servers.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	backups, err := h.backups.List(name)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if backups == nil {
		backups = []backup.Info{}
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(backups)
}

func (h *Handler) handleRestoreBackup(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	backupName := r.PathValue("backup")

	if !h.servers.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	proc := h.processes.Get(name)
	wasRunning := proc != nil && proc.GetState() == process.StateRunning

	if wasRunning {
		if err := h.processes.Stop(name); err != nil {
			writeError(w, "failed to stop server: "+err.Error(), http.StatusConflict)
			return
		}
	}

	if err := h.backups.Restore(name, backupName); err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	resp := map[string]any{
		"restored":    backupName,
		"was_running": wasRunning,
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(resp)
}

func (h *Handler) handleDeleteBackup(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	backupName := r.PathValue("backup")

	if !h.servers.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	if err := h.backups.Delete(name, backupName); err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (h *Handler) handlePruneBackups(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	if !h.servers.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	cfg, err := h.servers.LoadConfig(name)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	policy := backup.RetentionPolicy{
		KeepLast:   cfg.Backup.Retention.KeepLast,
		KeepDays:   cfg.Backup.Retention.KeepDays,
		MinBackups: cfg.Backup.Retention.MinBackups,
	}

	var req pruneRequest
	if r.ContentLength > 0 {
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeError(w, err.Error(), http.StatusBadRequest)
			return
		}
		if req.KeepLast > 0 {
			policy.KeepLast = req.KeepLast
		}
		if req.KeepDays > 0 {
			policy.KeepDays = req.KeepDays
		}
		if req.MinBackups > 0 {
			policy.MinBackups = req.MinBackups
		}
	}

	deleted, err := h.backups.Prune(name, policy)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(pruneResponse{
		Deleted: len(deleted),
		Names:   deleted,
	})
}
