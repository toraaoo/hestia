//go:build !windows

package daemon

import (
	"net"
	"os"

	"github.com/toraaoo/hestia/internal/log"
)

func listen(path string) (net.Listener, error) {
	return net.Listen("unix", path)
}

func cleanupListener(path string) {
	if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
		log.Warn("remove socket", "path", path, "err", err)
	}
}
