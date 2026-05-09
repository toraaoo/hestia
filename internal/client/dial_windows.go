//go:build windows

package client

import (
	"context"
	"net"

	"github.com/Microsoft/go-winio"
)

const pipeName = `\\.\pipe\hestia`

func dial(ctx context.Context, _ string) (net.Conn, error) {
	return winio.DialPipeContext(ctx, pipeName)
}
