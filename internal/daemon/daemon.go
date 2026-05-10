package daemon

import (
	"context"
	"net/http"
	"os"
	"os/signal"

	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/daemon/api"
	"github.com/toraaoo/hestia/internal/daemon/process"
)

func Run() error {
	cfg, err := config.Load()
	if err != nil {
		return err
	}

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

	go srv.Serve(ln) //nolint:errcheck

	select {
	case <-ctx.Done():
	case <-shutdownCh:
	}
	return srv.Shutdown(context.Background())
}
