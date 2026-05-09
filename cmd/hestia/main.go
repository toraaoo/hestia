package main

import (
	"os"

	"github.com/toraaoo/hestia/internal/cli"
)

func main() {
	if err := cli.Execute(); err != nil {
		os.Exit(1)
	}
}
