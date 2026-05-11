package api

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/jre"
	"github.com/toraaoo/hestia/internal/progress"
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
	Jar     string `json:"jar,omitempty"`

	// RCON
	RCONEnabled  *bool  `json:"rcon_enabled,omitempty"`
	RCONPassword string `json:"rcon_password,omitempty"`
	RCONPort     int    `json:"rcon_port,omitempty"`

	// World
	WorldName  string `json:"world_name,omitempty"`
	Seed       string `json:"seed,omitempty"`
	Gamemode   string `json:"gamemode,omitempty"`
	Difficulty string `json:"difficulty,omitempty"`
	MaxPlayers int    `json:"max_players,omitempty"`
	MOTD       string `json:"motd,omitempty"`
}

type serverInfo struct {
	Name    string        `json:"name"`
	Version string        `json:"version"`
	Port    int           `json:"port"`
	State   process.State `json:"state"`
	PID     int           `json:"pid,omitempty"`
}

func applyRequestToConfig(cfg *server.Config, req createRequest) {
	if req.Jar != "" {
		cfg.Jar = req.Jar
	}
	if req.Memory != "" {
		cfg.Memory = req.Memory
	}
	if req.Port != 0 {
		cfg.Port = req.Port
	}

	// RCON
	if req.RCONEnabled != nil {
		cfg.RCON.Enabled = *req.RCONEnabled
	}
	if req.RCONPassword != "" {
		cfg.RCON.Password = req.RCONPassword
	}
	if req.RCONPort != 0 {
		cfg.RCON.Port = req.RCONPort
	}

	// World
	if req.WorldName != "" {
		cfg.World.Name = req.WorldName
	}
	if req.Seed != "" {
		cfg.World.Seed = req.Seed
	}
	if req.Gamemode != "" {
		cfg.World.Gamemode = req.Gamemode
	}
	if req.Difficulty != "" {
		cfg.World.Difficulty = req.Difficulty
	}
	if req.MaxPlayers != 0 {
		cfg.World.MaxPlayers = req.MaxPlayers
	}
	if req.MOTD != "" {
		cfg.World.MOTD = req.MOTD
	}
}

func handleCreateServer(w http.ResponseWriter, r *http.Request) {
	var req createRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, err.Error(), http.StatusBadRequest)
		return
	}

	if req.Name == "" || req.Version == "" {
		writeError(w, "name and version required", http.StatusBadRequest)
		return
	}

	if r.Header.Get("Accept") == "text/event-stream" {
		handleCreateServerSSE(w, r, req)
		return
	}

	cfg, err := server.Create(req.Name, req.Version)
	if err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}

	applyRequestToConfig(cfg, req)
	if err := cfg.Save(); err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	provider, err := jar.GetProvider(cfg.Jar)
	if err != nil {
		_ = server.Delete(req.Name)
		writeError(w, "unsupported jar type: "+cfg.Jar, http.StatusBadRequest)
		return
	}

	jarPath := server.JarPath(req.Name)
	if err := provider.DownloadServer(req.Version, jarPath, nil); err != nil {
		_ = server.Delete(req.Name)
		writeError(w, "download server: "+err.Error(), http.StatusInternalServerError)
		return
	}

	javaVersion, _ := provider.GetJavaVersion(req.Version)
	if javaVersion > 0 && !jre.IsInstalled(javaVersion) {
		if err := jre.Download(javaVersion, nil); err != nil {
			_ = server.Delete(req.Name)
			writeError(w, "download jre: "+err.Error(), http.StatusInternalServerError)
			return
		}
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(cfg)
}

func handleCreateServerSSE(w http.ResponseWriter, _ *http.Request, req createRequest) {
	sse, err := NewSSEWriter(w)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	cfg, err := server.Create(req.Name, req.Version)
	if err != nil {
		_ = sse.WriteError(err.Error())
		return
	}

	applyRequestToConfig(cfg, req)
	if err := cfg.Save(); err != nil {
		_ = server.Delete(req.Name)
		_ = sse.WriteError("save config: " + err.Error())
		return
	}

	provider, err := jar.GetProvider(cfg.Jar)
	if err != nil {
		_ = server.Delete(req.Name)
		_ = sse.WriteError("unsupported jar type: " + cfg.Jar)
		return
	}

	cb := func(evt progress.Event) { _ = sse.WriteEvent(evt) }

	jarPath := server.JarPath(req.Name)
	if err := provider.DownloadServer(req.Version, jarPath, cb); err != nil {
		_ = server.Delete(req.Name)
		_ = sse.WriteError("download server: " + err.Error())
		return
	}

	javaVersion, _ := provider.GetJavaVersion(req.Version)
	if javaVersion > 0 {
		if jre.IsInstalled(javaVersion) {
			cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryJRE, Message: "cached"})
			cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryExtract, Message: "skipped"})
		} else {
			if err := jre.Download(javaVersion, cb); err != nil {
				_ = server.Delete(req.Name)
				_ = sse.WriteError("download jre: " + err.Error())
				return
			}
		}
	}

	_ = sse.WriteDone(cfg)
}

func handleListServers(w http.ResponseWriter, r *http.Request) {
	names, err := server.List()
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
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
	_ = json.NewEncoder(w).Encode(servers)
}

func handleGetServer(w http.ResponseWriter, r *http.Request) {
	name := strings.TrimPrefix(r.URL.Path, "/servers/")
	name = strings.Split(name, "/")[0]

	cfg, err := server.LoadConfig(name)
	if err != nil {
		writeError(w, "server not found", http.StatusNotFound)
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
	_ = json.NewEncoder(w).Encode(resp)
}

func handleDeleteServer(w http.ResponseWriter, r *http.Request) {
	name := strings.TrimPrefix(r.URL.Path, "/servers/")

	if proc := procManager.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		writeError(w, "server must be stopped first", http.StatusConflict)
		return
	}

	if err := server.Delete(name); err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func handleStartServer(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/start")
	if err := procManager.Start(name); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func handleStopServer(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/stop")
	if err := procManager.Stop(name); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func handleRestartServer(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/restart")

	if proc := procManager.Get(name); proc != nil && proc.GetState() == process.StateRunning {
		if err := procManager.Stop(name); err != nil {
			writeError(w, err.Error(), http.StatusConflict)
			return
		}
	}

	if err := procManager.Start(name); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func extractServerName(path, suffix string) string {
	path = strings.TrimPrefix(path, "/servers/")
	path = strings.TrimSuffix(path, suffix)
	return path
}
