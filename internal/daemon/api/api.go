package api

import (
	"encoding/json"
	"net/http"

	"github.com/toraaoo/hestia/internal/backup"
	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/jre"
	"github.com/toraaoo/hestia/internal/server"
)

type Shutdown interface {
	Trigger() bool
}

type HandlerDeps struct {
	Shutdown  Shutdown
	Servers   *server.Store
	Processes *process.Manager
	Loaders   *jar.Registry
	JRE       *jre.Manager
	Backups   *backup.Service
	Scheduler *backup.Scheduler
}

type Handler struct {
	shutdown  Shutdown
	servers   *server.Store
	processes *process.Manager
	loaders   *jar.Registry
	jre       *jre.Manager
	backups   *backup.Service
	scheduler *backup.Scheduler
}

func NewHandler(deps HandlerDeps) *Handler {
	return &Handler{
		shutdown:  deps.Shutdown,
		servers:   deps.Servers,
		processes: deps.Processes,
		loaders:   deps.Loaders,
		jre:       deps.JRE,
		backups:   deps.Backups,
		scheduler: deps.Scheduler,
	}
}

func (h *Handler) RegisterRoutes(mux *http.ServeMux) {
	mux.HandleFunc("GET /ping", h.ping)
	mux.HandleFunc("POST /shutdown", h.handleShutdown)
	mux.HandleFunc("GET /versions", h.handleVersions)

	mux.HandleFunc("POST /servers", h.handleCreateServer)
	mux.HandleFunc("GET /servers", h.handleListServers)
	mux.HandleFunc("GET /servers/{name}", h.handleGetServer)
	mux.HandleFunc("DELETE /servers/{name}", h.handleDeleteServer)

	mux.HandleFunc("POST /servers/{name}/start", h.handleStartServer)
	mux.HandleFunc("POST /servers/{name}/stop", h.handleStopServer)
	mux.HandleFunc("POST /servers/{name}/restart", h.handleRestartServer)
	mux.HandleFunc("POST /servers/{name}/upgrade", h.handleUpgradeServer)

	mux.HandleFunc("GET /servers/{name}/logs", h.handleLogs)
	mux.HandleFunc("POST /servers/{name}/console", h.handleConsole)

	mux.HandleFunc("GET /servers/{name}/config", h.handleGetConfig)
	mux.HandleFunc("PUT /servers/{name}/config", h.handleUpdateConfig)

	mux.HandleFunc("POST /servers/{name}/backup", h.handleCreateBackup)
	mux.HandleFunc("GET /servers/{name}/backups", h.handleListBackups)
	mux.HandleFunc("POST /servers/{name}/backups/{backup}/restore", h.handleRestoreBackup)
	mux.HandleFunc("DELETE /servers/{name}/backups/{backup}", h.handleDeleteBackup)
	mux.HandleFunc("POST /servers/{name}/backups/prune", h.handlePruneBackups)
}

func (h *Handler) ping(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func (h *Handler) handleShutdown(w http.ResponseWriter, _ *http.Request) {
	w.WriteHeader(http.StatusOK)
	if h.shutdown != nil {
		h.shutdown.Trigger()
	}
}

type apiError struct {
	Error string `json:"error"`
}

func writeError(w http.ResponseWriter, msg string, code int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(apiError{Error: msg})
}
