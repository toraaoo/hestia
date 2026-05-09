package main

import (
	"fmt"
	"os"

	"github.com/toraaoo/hestia/internal/daemon"
)

func main() {
	if err := daemon.Run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
