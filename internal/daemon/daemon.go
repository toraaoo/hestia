package daemon

import (
	"context"
	"net/http"
	"os"
	"os/signal"
	"time"

	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/daemon/api"
	"github.com/toraaoo/hestia/internal/daemon/process"
	"github.com/toraaoo/hestia/internal/log"
)

func Run() error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}

	log.Init(cfg.Daemon.LogLevel)
	log.Info("starting hestiad")

	if err := os.MkdirAll(config.DefaultDir(), 0o700); err != nil {
		return err
	}

	pm := process.NewManager()
	mux := http.NewServeMux()
	shutdownCh := make(chan struct{})
	api.Register(mux, shutdownCh, pm)

	ln, err := listen(cfg.Daemon.Sock)
	if err != nil {
		return err
	}
	defer cleanupListener(cfg.Daemon.Sock)

	srv := &http.Server{Handler: mux}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt)
	defer stop()

	srvErr := make(chan error, 1)
	go func() {
		if err := srv.Serve(ln); err != nil && err != http.ErrServerClosed {
			srvErr <- err
		}
	}()

	select {
	case err := <-srvErr:
		return err
	case <-ctx.Done():
		log.Info("shutting down", "reason", "signal")
	case <-shutdownCh:
		log.Info("shutting down", "reason", "request")
	}

	pm.StopAll()

	shutCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	return srv.Shutdown(shutCtx)
}
