package app

import (
	"context"
	"net/http"
	"time"

	"github.com/toraaoo/hestia/internal/client"
	"github.com/toraaoo/hestia/internal/config"
	"github.com/toraaoo/hestia/internal/download"
	"github.com/toraaoo/hestia/internal/jar"
	"github.com/toraaoo/hestia/internal/jar/loaders"
)

type CLIApp struct {
	Paths   Paths
	Config  *config.Config
	Client  *client.Client
	Loaders *jar.Registry
}

func NewCLIApp(_ context.Context) (*CLIApp, error) {
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
	return &CLIApp{
		Paths:   paths,
		Config:  cfg,
		Client:  client.New(cfg.Daemon.Sock),
		Loaders: loaders.NewRegistry(httpClient, downloadClient),
	}, nil
}
