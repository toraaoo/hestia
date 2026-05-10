package jre

import (
	"archive/tar"
	"compress/gzip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/toraaoo/hestia/internal/httpc"
)

func adoptiumArch() string {
	switch runtime.GOARCH {
	case "amd64":
		return "x64"
	case "arm64":
		return "aarch64"
	case "386":
		return "x86"
	default:
		return runtime.GOARCH
	}
}

func adoptiumOS() string {
	switch runtime.GOOS {
	case "darwin":
		return "mac"
	default:
		return runtime.GOOS
	}
}

func downloadURL(majorVersion int) string {
	return fmt.Sprintf(
		"https://api.adoptium.net/v3/binary/latest/%d/ga/%s/%s/jre/hotspot/normal/eclipse",
		majorVersion, adoptiumOS(), adoptiumArch(),
	)
}

func Download(majorVersion int) error {
	url := downloadURL(majorVersion)
	resp, err := httpc.GetDownload(url)
	if err != nil {
		return fmt.Errorf("fetch jre: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return fmt.Errorf("adoptium api: %s", resp.Status)
	}

	tmpFile, err := os.CreateTemp("", "jre-*.tar.gz")
	if err != nil {
		return fmt.Errorf("create temp file: %w", err)
	}
	tmpPath := tmpFile.Name()
	defer os.Remove(tmpPath)

	if _, err := io.Copy(tmpFile, resp.Body); err != nil {
		tmpFile.Close()
		return fmt.Errorf("download jre: %w", err)
	}
	if err := tmpFile.Close(); err != nil {
		return fmt.Errorf("close temp file: %w", err)
	}

	f, err := os.Open(tmpPath)
	if err != nil {
		return fmt.Errorf("open temp file: %w", err)
	}
	defer f.Close()

	destDir := versionDir(majorVersion)
	if err := os.MkdirAll(destDir, 0755); err != nil {
		return fmt.Errorf("create jre dir: %w", err)
	}

	if err := extractTarGz(f, destDir); err != nil {
		os.RemoveAll(destDir)
		return fmt.Errorf("extract jre: %w", err)
	}
	return nil
}

func extractTarGz(r io.Reader, destDir string) error {
	gz, err := gzip.NewReader(r)
	if err != nil {
		return err
	}
	defer gz.Close()

	tr := tar.NewReader(gz)
	var stripPrefix string

	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}

		if stripPrefix == "" {
			parts := strings.SplitN(hdr.Name, "/", 2)
			if len(parts) > 0 {
				stripPrefix = parts[0] + "/"
			}
		}

		name := strings.TrimPrefix(hdr.Name, stripPrefix)
		if name == "" {
			continue
		}

		target := filepath.Join(destDir, name)

		switch hdr.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, os.FileMode(hdr.Mode)); err != nil {
				return err
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
				return err
			}
			f, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, os.FileMode(hdr.Mode))
			if err != nil {
				return err
			}
			if _, err := io.Copy(f, tr); err != nil {
				f.Close()
				return err
			}
			f.Close()
		case tar.TypeSymlink:
			if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
				return err
			}
			os.Remove(target)
			if err := os.Symlink(hdr.Linkname, target); err != nil {
				return err
			}
		}
	}
	return nil
}
