package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"github.com/toraaoo/hestia/internal/backup"
	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/server"
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

func handleCreateBackup(w http.ResponseWriter, r *http.Request) {
	name := extractBackupServerName(r.URL.Path, "/backup")

	cfg, err := server.LoadConfig(name)
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

	proc := procManager.Get(name)
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

	info, err := backup.Create(opts)
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
		_, _ = backup.Prune(name, policy)
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(info)
}

func handleListBackups(w http.ResponseWriter, r *http.Request) {
	name := extractBackupServerName(r.URL.Path, "/backups")

	if !server.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	backups, err := backup.List(name)
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

func handleRestoreBackup(w http.ResponseWriter, r *http.Request) {
	parts := strings.Split(strings.TrimPrefix(r.URL.Path, "/servers/"), "/")
	if len(parts) < 3 {
		writeError(w, "invalid path", http.StatusBadRequest)
		return
	}
	name := parts[0]
	backupName := parts[2]

	if !server.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	proc := procManager.Get(name)
	wasRunning := proc != nil && proc.GetState() == process.StateRunning

	if wasRunning {
		if err := procManager.Stop(name); err != nil {
			writeError(w, "failed to stop server: "+err.Error(), http.StatusConflict)
			return
		}
	}

	if err := backup.Restore(name, backupName); err != nil {
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

func handleDeleteBackup(w http.ResponseWriter, r *http.Request) {
	parts := strings.Split(strings.TrimPrefix(r.URL.Path, "/servers/"), "/")
	if len(parts) < 2 {
		writeError(w, "invalid path", http.StatusBadRequest)
		return
	}
	name := parts[0]
	backupName := parts[2]

	if !server.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	if err := backup.Delete(name, backupName); err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func handlePruneBackups(w http.ResponseWriter, r *http.Request) {
	name := extractBackupServerName(r.URL.Path, "/backups/prune")

	if !server.Exists(name) {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	cfg, err := server.LoadConfig(name)
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

	deleted, err := backup.Prune(name, policy)
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

func extractBackupServerName(path, suffix string) string {
	path = strings.TrimPrefix(path, "/servers/")
	path = strings.TrimSuffix(path, suffix)
	return path
}
