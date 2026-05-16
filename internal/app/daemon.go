package app

import (
	"context"
	"net/http"
	"time"

	"github.com/toraaoo/hestia/internal/backup"
	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/daemon/api"
	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/jar/loaders"
	"github.com/toraaoo/hestia/internal/jre"
	"github.com/toraaoo/hestia/internal/server"
)

type DaemonApp struct {
	Paths     Paths
	Config    *config.Config
	HTTP      *http.Server
	Handler   *api.Handler
	Processes *process.Manager
	Backups   *backup.Service
	Scheduler *backup.Scheduler
	Shutdown  *Shutdown
	Servers   *server.Store
	JRE       *jre.Manager
	Providers *jar.Registry
}

type processManagerState struct {
	pm    *process.Manager
	store *server.Store
}

func (s *processManagerState) IsRunning(serverName string) bool {
	proc := s.pm.Get(serverName)
	return proc != nil && proc.GetState() == process.StateRunning
}

func (s *processManagerState) GetRCONInfo(serverName string) (port int, password string, enabled bool) {
	cfg, err := s.store.LoadConfig(serverName)
	if err != nil {
		return 0, "", false
	}
	return cfg.RCON.Port, cfg.RCON.Password, cfg.RCON.Enabled
}

func NewDaemonApp(_ context.Context) (*DaemonApp, error) {
	paths, err := ResolvePaths()
	if err != nil {
		return nil, err
	}
	cfg, err := LoadConfig(paths)
	if err != nil {
		return nil, err
	}

	httpClient := download.NewClient(&http.Client{Timeout: 30 * time.Second}, "hestia/1.0")
	downloadClient := download.NewClient(&http.Client{Timeout: 10 * time.Minute}, "hestia/1.0")
	providers := loaders.NewRegistry(httpClient, downloadClient)
	servers := server.NewStore(paths.ServersDir)
	jreManager := jre.NewManager(paths.JREDir, jre.NewDownloader(downloadClient))
	processes := process.NewManager(servers, jreManager, providers)
	backups := backup.NewService(servers, nil)
	scheduler := backup.NewScheduler(&processManagerState{pm: processes, store: servers}, servers, backups)
	shutdown := NewShutdown()

	handler := api.NewHandler(api.HandlerDeps{
		Shutdown:  shutdown,
		Servers:   servers,
		Processes: processes,
		Jars:      providers,
		JRE:       jreManager,
		Backups:   backups,
		Scheduler: scheduler,
	})
	mux := http.NewServeMux()
	handler.RegisterRoutes(mux)

	return &DaemonApp{
		Paths:     paths,
		Config:    cfg,
		HTTP:      &http.Server{Handler: mux},
		Handler:   handler,
		Processes: processes,
		Backups:   backups,
		Scheduler: scheduler,
		Shutdown:  shutdown,
		Servers:   servers,
		JRE:       jreManager,
		Providers: providers,
	}, nil
}
