package api

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/server"
)

var procManager *process.Manager

func SetProcessManager(m *process.Manager) {
	procManager = m
}

type createRequest struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Memory  string `json:"memory,omitempty"`
	Port    int    `json:"port,omitempty"`
}

type serverInfo struct {
	Name    string        `json:"name"`
	Version string        `json:"version"`
	Port    int           `json:"port"`
	State   process.State `json:"state"`
	PID     int           `json:"pid,omitempty"`
}

func handleCreateServer(w http.ResponseWriter, r *http.Request) {
	var req createRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	if req.Name == "" || req.Version == "" {
		http.Error(w, "name and version required", http.StatusBadRequest)
		return
	}

	cfg, err := server.Create(req.Name, req.Version)
	if err != nil {
		http.Error(w, err.Error(), http.StatusConflict)
		return
	}

	if req.Memory != "" {
		cfg.Memory = req.Memory
		cfg.Save()
	}

	jarPath := server.JarPath(req.Name)
	provider := jar.VanillaProvider{}
	if err := provider.DownloadServer(req.Version, jarPath); err != nil {
		server.Delete(req.Name)
		http.Error(w, "download server: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(cfg)
}

func handleListServers(w http.ResponseWriter, r *http.Request) {
	names, err := server.List()
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	servers := make([]serverInfo, 0, len(names))
	for _, name := range names {
		cfg, err := server.LoadConfig(name)
		if err != nil {
			continue
		}

		info := serverInfo{
			Name:    cfg.Name,
			Version: cfg.Version,
			Port:    cfg.Port,
			State:   process.StateStopped,
		}

		if proc := procManager.Get(name); proc != nil {
			info.State = proc.GetState()
			info.PID = proc.PID
		}

		servers = append(servers, info)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(servers)
}

func handleGetServer(w http.ResponseWriter, r *http.Request) {
	name := strings.TrimPrefix(r.URL.Path, "/servers/")
	name = strings.Split(name, "/")[0]

	cfg, err := server.LoadConfig(name)
	if err != nil {
		http.Error(w, "server not found", http.StatusNotFound)
		return
	}

	resp := struct {
		*server.Config
		State process.State `json:"state"`
		PID   int           `json:"pid,omitempty"`
	}{Config: cfg, State: process.StateStopped}

	if proc := procManager.Get(name); proc != nil {
		resp.State = proc.GetState()
		resp.PID = proc.PID
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
}

func handleDeleteServer(w http.ResponseWriter, r *http.Request) {
	name := strings.TrimPrefix(r.URL.Path, "/servers/")

	if proc := procManager.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		http.Error(w, "server must be stopped first", http.StatusConflict)
		return
	}

	if err := server.Delete(name); err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func handleStartServer(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/start")
	if err := procManager.Start(name); err != nil {
		http.Error(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func handleStopServer(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/stop")
	if err := procManager.Stop(name); err != nil {
		http.Error(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func handleRestartServer(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/restart")

	if proc := procManager.Get(name); proc != nil && proc.GetState() == process.StateRunning {
		if err := procManager.Stop(name); err != nil {
			http.Error(w, err.Error(), http.StatusConflict)
			return
		}
	}

	if err := procManager.Start(name); err != nil {
		http.Error(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func extractServerName(path, suffix string) string {
	path = strings.TrimPrefix(path, "/servers/")
	path = strings.TrimSuffix(path, suffix)
	return path
}
