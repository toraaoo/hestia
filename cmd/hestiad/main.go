package main

import (
	"fmt"
	"os"

	"github.com/toraaoo/hestia/internal/daemon"
	_ "github.com/toraaoo/hestia/internal/jar/providers"
)

func main() {
	if err := daemon.Run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
