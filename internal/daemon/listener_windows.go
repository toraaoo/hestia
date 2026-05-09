//go:build windows

package daemon

import (
	"net"

	"github.com/Microsoft/go-winio"
)

const pipeName = `\\.\pipe\hestia`

func listen(_ string) (net.Listener, error) {
	return winio.ListenPipe(pipeName, nil)
}

func cleanupListener(_ string) {
	// Named pipes clean up automatically when closed
}
