//go:build !windows

package daemon

import (
	"net"
	"os"
)

func listen(path string) (net.Listener, error) {
	return net.Listen("unix", path)
}

func cleanupListener(path string) {
	os.Remove(path)
}
