package api

import (
	"encoding/json"
	"net/http"

	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/progress"
	"github.com/toraaoo/hestia/internal/server"
)

type createRequest struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Memory  string `json:"memory,omitempty"`
	Port    int    `json:"port,omitempty"`
	Loader  string `json:"loader,omitempty"`

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
	Loader  string        `json:"loader"`
	Port    int           `json:"port"`
	State   process.State `json:"state"`
	PID     int           `json:"pid,omitempty"`
}

func applyRequestToConfig(cfg *server.Config, req createRequest) {
	if req.Loader != "" {
		cfg.Loader = req.Loader
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

func (h *Handler) handleCreateServer(w http.ResponseWriter, r *http.Request) {
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
		h.handleCreateServerSSE(w, r, req)
		return
	}

	cfg, err := h.servers.Create(req.Name, req.Version)
	if err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}

	applyRequestToConfig(cfg, req)
	if err := h.servers.SaveConfig(cfg); err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	provider, err := h.loaders.GetLoader(cfg.Loader)
	if err != nil {
		_ = h.servers.Delete(req.Name)
		writeError(w, "unsupported loader: "+cfg.Loader, http.StatusBadRequest)
		return
	}

	jarPath := h.servers.JarPath(req.Name)
	if err := provider.DownloadServer(req.Version, jarPath, nil); err != nil {
		_ = h.servers.Delete(req.Name)
		writeError(w, "download server: "+err.Error(), http.StatusInternalServerError)
		return
	}

	javaVersion, _ := provider.GetJavaVersion(req.Version)
	if javaVersion > 0 && !h.jre.IsInstalled(javaVersion) {
		if err := h.jre.Download(javaVersion, nil); err != nil {
			_ = h.servers.Delete(req.Name)
			writeError(w, "download jre: "+err.Error(), http.StatusInternalServerError)
			return
		}
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(cfg)
}

func (h *Handler) handleCreateServerSSE(w http.ResponseWriter, _ *http.Request, req createRequest) {
	sse, err := NewSSEWriter(w)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	cfg, err := h.servers.Create(req.Name, req.Version)
	if err != nil {
		_ = sse.WriteError(err.Error())
		return
	}

	applyRequestToConfig(cfg, req)
	if err := h.servers.SaveConfig(cfg); err != nil {
		_ = h.servers.Delete(req.Name)
		_ = sse.WriteError("save config: " + err.Error())
		return
	}

	provider, err := h.loaders.GetLoader(cfg.Loader)
	if err != nil {
		_ = h.servers.Delete(req.Name)
		_ = sse.WriteError("unsupported loader: " + cfg.Loader)
		return
	}

	cb := func(evt progress.Event) { _ = sse.WriteEvent(evt) }

	jarPath := h.servers.JarPath(req.Name)
	if err := provider.DownloadServer(req.Version, jarPath, cb); err != nil {
		_ = h.servers.Delete(req.Name)
		_ = sse.WriteError("download server: " + err.Error())
		return
	}

	javaVersion, _ := provider.GetJavaVersion(req.Version)
	if javaVersion > 0 {
		if h.jre.IsInstalled(javaVersion) {
			cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryJRE, Message: "cached"})
			cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryExtract, Message: "skipped"})
		} else {
			if err := h.jre.Download(javaVersion, cb); err != nil {
				_ = h.servers.Delete(req.Name)
				_ = sse.WriteError("download jre: " + err.Error())
				return
			}
		}
	}

	_ = sse.WriteDone(cfg)
}

func (h *Handler) handleListServers(w http.ResponseWriter, r *http.Request) {
	names, err := h.servers.List()
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	servers := make([]serverInfo, 0, len(names))
	for _, name := range names {
		cfg, err := h.servers.LoadConfig(name)
		if err != nil {
			continue
		}

		info := serverInfo{
			Name:    cfg.Name,
			Version: cfg.Version,
			Loader:  cfg.Loader,
			Port:    cfg.Port,
			State:   process.StateStopped,
		}

		if proc := h.processes.Get(name); proc != nil {
			info.State = proc.GetState()
			info.PID = proc.PID
		}

		servers = append(servers, info)
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(servers)
}

func (h *Handler) handleGetServer(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	cfg, err := h.servers.LoadConfig(name)
	if err != nil {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	resp := struct {
		*server.Config
		State process.State `json:"state"`
		PID   int           `json:"pid,omitempty"`
	}{Config: cfg, State: process.StateStopped}

	if proc := h.processes.Get(name); proc != nil {
		resp.State = proc.GetState()
		resp.PID = proc.PID
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(resp)
}

func (h *Handler) handleDeleteServer(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	if proc := h.processes.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		writeError(w, "server must be stopped first", http.StatusConflict)
		return
	}

	if err := h.servers.Delete(name); err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

func (h *Handler) handleStartServer(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	if err := h.processes.Start(name); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func (h *Handler) handleStopServer(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	if err := h.processes.Stop(name); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

func (h *Handler) handleRestartServer(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	if proc := h.processes.Get(name); proc != nil && proc.GetState() == process.StateRunning {
		if err := h.processes.Stop(name); err != nil {
			writeError(w, err.Error(), http.StatusConflict)
			return
		}
	}

	if err := h.processes.Start(name); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}
	w.WriteHeader(http.StatusAccepted)
}

type upgradeRequest struct {
	Version  string `json:"version"`
	NoBackup bool   `json:"no_backup,omitempty"`
}

func (h *Handler) handleUpgradeServer(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	var req upgradeRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, err.Error(), http.StatusBadRequest)
		return
	}

	if req.Version == "" {
		writeError(w, "version required", http.StatusBadRequest)
		return
	}

	cfg, err := h.servers.LoadConfig(name)
	if err != nil {
		writeError(w, "server not found", http.StatusNotFound)
		return
	}

	if r.Header.Get("Accept") == "text/event-stream" {
		h.handleUpgradeServerSSE(w, r, name, cfg, req)
		return
	}

	if proc := h.processes.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		if err := h.processes.Stop(name); err != nil {
			writeError(w, "stop server: "+err.Error(), http.StatusConflict)
			return
		}
	}

	var backupPath string
	if !req.NoBackup {
		backupPath, err = h.servers.BackupJar(name)
		if err != nil {
			writeError(w, "backup jar: "+err.Error(), http.StatusInternalServerError)
			return
		}
		_ = h.servers.PruneJarBackups(name, 3)
	}

	provider, err := h.loaders.GetLoader(cfg.Loader)
	if err != nil {
		writeError(w, "unsupported loader: "+cfg.Loader, http.StatusBadRequest)
		return
	}

	jarPath := h.servers.JarPath(name)
	if err := provider.DownloadServer(req.Version, jarPath, nil); err != nil {
		writeError(w, "download server: "+err.Error(), http.StatusInternalServerError)
		return
	}

	javaVersion, _ := provider.GetJavaVersion(req.Version)
	if javaVersion > 0 && !h.jre.IsInstalled(javaVersion) {
		if err := h.jre.Download(javaVersion, nil); err != nil {
			writeError(w, "download jre: "+err.Error(), http.StatusInternalServerError)
			return
		}
	}

	cfg.Version = req.Version
	if err := h.servers.SaveConfig(cfg); err != nil {
		writeError(w, "save config: "+err.Error(), http.StatusInternalServerError)
		return
	}

	resp := map[string]any{
		"version":     req.Version,
		"backup_path": backupPath,
	}
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(resp)
}

func (h *Handler) handleUpgradeServerSSE(w http.ResponseWriter, _ *http.Request, name string, cfg *server.Config, req upgradeRequest) {
	sse, err := NewSSEWriter(w)
	if err != nil {
		writeError(w, err.Error(), http.StatusInternalServerError)
		return
	}

	cb := func(evt progress.Event) { _ = sse.WriteEvent(evt) }

	if proc := h.processes.Get(name); proc != nil && proc.GetState() != process.StateStopped {
		if err := h.processes.Stop(name); err != nil {
			_ = sse.WriteError("stop server: " + err.Error())
			return
		}
	}

	var backupPath string
	if !req.NoBackup {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryBackup, Message: "backing up server.jar"})
		backupPath, err = h.servers.BackupJar(name)
		if err != nil {
			_ = sse.WriteError("backup jar: " + err.Error())
			return
		}
		_ = h.servers.PruneJarBackups(name, 3)
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryBackup, Message: backupPath})
	}

	provider, err := h.loaders.GetLoader(cfg.Loader)
	if err != nil {
		_ = sse.WriteError("unsupported loader: " + cfg.Loader)
		return
	}

	jarPath := h.servers.JarPath(name)
	if err := provider.DownloadServer(req.Version, jarPath, cb); err != nil {
		_ = sse.WriteError("download server: " + err.Error())
		return
	}

	javaVersion, _ := provider.GetJavaVersion(req.Version)
	if javaVersion > 0 {
		if h.jre.IsInstalled(javaVersion) {
			cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryJRE, Message: "cached"})
			cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryExtract, Message: "skipped"})
		} else {
			if err := h.jre.Download(javaVersion, cb); err != nil {
				_ = sse.WriteError("download jre: " + err.Error())
				return
			}
		}
	}

	cfg.Version = req.Version
	if err := h.servers.SaveConfig(cfg); err != nil {
		_ = sse.WriteError("save config: " + err.Error())
		return
	}

	result := map[string]any{
		"version":     req.Version,
		"backup_path": backupPath,
	}
	_ = sse.WriteDone(result)
}
