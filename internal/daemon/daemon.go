package daemon

import (
	"context"
	"errors"
	"net"
	"net/http"
	"os"
	"os/signal"
	"time"

	"github.com/toraaoo/hestia/internal/app"
	"github.com/toraaoo/hestia/internal/log"
)

type Daemon struct {
	app      *app.DaemonApp
	listener net.Listener
	server   *http.Server
}

func New(app *app.DaemonApp) *Daemon {
	return &Daemon{app: app, server: app.HTTP}
}

func (d *Daemon) Run(ctx context.Context) error {
	log.Init(d.app.Config.Daemon.LogLevel)
	log.Info("starting hestiad")

	if err := os.MkdirAll(d.app.Paths.DataDir, 0o700); err != nil {
		return err
	}

	if err := d.app.Scheduler.LoadSchedules(); err != nil {
		log.Warn("failed to load backup schedules", "error", err)
	}
	d.app.Scheduler.Start()
	defer d.app.Scheduler.Stop()

	ln, err := listen(d.app.Config.Daemon.Sock)
	if err != nil {
		return err
	}
	d.listener = ln
	defer cleanupListener(d.app.Config.Daemon.Sock)

	ctx, stop := signal.NotifyContext(ctx, os.Interrupt)
	defer stop()

	srvErr := make(chan error, 1)
	go func() {
		if err := d.server.Serve(ln); err != nil && !errors.Is(err, http.ErrServerClosed) {
			srvErr <- err
		}
	}()

	select {
	case err := <-srvErr:
		return err
	case <-ctx.Done():
		log.Info("shutting down", "reason", "signal")
	case <-d.app.Shutdown.Done():
		log.Info("shutting down", "reason", "request")
	}

	d.app.Processes.StopAll()

	shutCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	return d.Shutdown(shutCtx)
}

func (d *Daemon) Shutdown(ctx context.Context) error {
	return d.server.Shutdown(ctx)
}

func Run() error {
	ctx := context.Background()
	daemonApp, err := app.NewDaemonApp(ctx)
	if err != nil {
		return err
	}
	return New(daemonApp).Run(ctx)
}
