package api

import (
	"encoding/json"
	"net/http"

	"github.com/toraaoo/hestia/internal/daemon/process"
)

var shutdownCh chan struct{}

func Register(mux *http.ServeMux, shutdown chan struct{}, pm *process.Manager) {
	shutdownCh = shutdown
	SetProcessManager(pm)

	mux.HandleFunc("GET /ping", ping)
	mux.HandleFunc("POST /shutdown", handleShutdown)
	mux.HandleFunc("GET /versions", handleVersions)

	mux.HandleFunc("POST /servers", handleCreateServer)
	mux.HandleFunc("GET /servers", handleListServers)
	mux.HandleFunc("GET /servers/{name}", handleGetServer)
	mux.HandleFunc("DELETE /servers/{name}", handleDeleteServer)

	mux.HandleFunc("POST /servers/{name}/start", handleStartServer)
	mux.HandleFunc("POST /servers/{name}/stop", handleStopServer)
	mux.HandleFunc("POST /servers/{name}/restart", handleRestartServer)
	mux.HandleFunc("POST /servers/{name}/upgrade", handleUpgradeServer)

	mux.HandleFunc("GET /servers/{name}/logs", handleLogs)
	mux.HandleFunc("POST /servers/{name}/console", handleConsole)

	mux.HandleFunc("GET /servers/{name}/config", handleGetConfig)
	mux.HandleFunc("PUT /servers/{name}/config", handleUpdateConfig)

	mux.HandleFunc("POST /servers/{name}/backup", handleCreateBackup)
	mux.HandleFunc("GET /servers/{name}/backups", handleListBackups)
	mux.HandleFunc("POST /servers/{name}/backups/{backup}/restore", handleRestoreBackup)
	mux.HandleFunc("DELETE /servers/{name}/backups/{backup}", handleDeleteBackup)
	mux.HandleFunc("POST /servers/{name}/backups/prune", handlePruneBackups)
}

func ping(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func handleShutdown(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
	close(shutdownCh)
}

type apiError struct {
	Error string `json:"error"`
}

func writeError(w http.ResponseWriter, msg string, code int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(apiError{Error: msg})
}
