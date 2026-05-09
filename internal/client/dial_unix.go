//go:build !windows

package client

import (
	"context"
	"net"
)

func dial(ctx context.Context, path string) (net.Conn, error) {
	return (&net.Dialer{}).DialContext(ctx, "unix", path)
}
