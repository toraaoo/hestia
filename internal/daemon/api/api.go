package api

import (
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

	mux.HandleFunc("GET /servers/{name}/logs", handleLogs)
	mux.HandleFunc("POST /servers/{name}/console", handleConsole)

	mux.HandleFunc("GET /servers/{name}/config", handleGetConfig)
	mux.HandleFunc("PUT /servers/{name}/config", handleUpdateConfig)
}

func ping(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func handleShutdown(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	close(shutdownCh)
}
