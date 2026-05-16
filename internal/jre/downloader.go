package jre

import (
	"archive/tar"
	"archive/zip"
	"compress/gzip"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"

	"github.com/toraaoo/hestia/internal/progress"
)

type HTTPClient interface {
	Get(url string) (*http.Response, error)
}

type Downloader struct {
	http HTTPClient
}

func NewDownloader(httpClient HTTPClient) *Downloader {
	if httpClient == nil {
		httpClient = defaultHTTPClient{}
	}
	return &Downloader{http: httpClient}
}

type defaultHTTPClient struct{}

func (defaultHTTPClient) Get(url string) (*http.Response, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", "hestia/1.0")
	return http.DefaultClient.Do(req)
}

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

func (d *Downloader) Download(majorVersion int, destDir string, cb progress.Callback) error {
	url := downloadURL(majorVersion)

	if cb != nil {
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryJRE, Message: "downloading"})
	}

	resp, err := d.http.Get(url)
	if err != nil {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryJRE, Error: err.Error()})
		}
		return fmt.Errorf("fetch jre: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode != 200 {
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryJRE, Error: resp.Status})
		}
		return fmt.Errorf("adoptium api: %s", resp.Status)
	}

	tmpFile, err := os.CreateTemp("", "jre-*.tar.gz")
	if err != nil {
		return fmt.Errorf("create temp file: %w", err)
	}
	tmpPath := tmpFile.Name()
	defer func() { _ = os.Remove(tmpPath) }()

	total := resp.ContentLength
	var downloaded int64
	buf := make([]byte, 32*1024)
	for {
		n, readErr := resp.Body.Read(buf)
		if n > 0 {
			if _, writeErr := tmpFile.Write(buf[:n]); writeErr != nil {
				_ = tmpFile.Close()
				return fmt.Errorf("write temp: %w", writeErr)
			}
			downloaded += int64(n)
			if cb != nil {
				cb(progress.Event{Type: progress.EventProgress, Category: progress.CategoryJRE, Current: downloaded, Total: total})
			}
		}
		if readErr == io.EOF {
			break
		}
		if readErr != nil {
			_ = tmpFile.Close()
			return fmt.Errorf("download jre: %w", readErr)
		}
	}
	if err := tmpFile.Close(); err != nil {
		return fmt.Errorf("close temp file: %w", err)
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryJRE})
		cb(progress.Event{Type: progress.EventStart, Category: progress.CategoryExtract, Message: "extracting"})
	}

	f, err := os.Open(tmpPath)
	if err != nil {
		return fmt.Errorf("open temp file: %w", err)
	}
	defer func() { _ = f.Close() }()

	if err := os.MkdirAll(destDir, 0755); err != nil {
		return fmt.Errorf("create jre dir: %w", err)
	}

	if err := extractArchive(f, destDir); err != nil {
		_ = os.RemoveAll(destDir)
		if cb != nil {
			cb(progress.Event{Type: progress.EventError, Category: progress.CategoryExtract, Error: err.Error()})
		}
		return fmt.Errorf("extract jre: %w", err)
	}

	if cb != nil {
		cb(progress.Event{Type: progress.EventComplete, Category: progress.CategoryExtract})
	}
	return nil
}

func extractArchive(f *os.File, destDir string) error {
	if _, err := f.Seek(0, io.SeekStart); err != nil {
		return err
	}
	var magic [4]byte
	_, err := io.ReadFull(f, magic[:])
	if err != nil {
		return err
	}
	if _, err := f.Seek(0, io.SeekStart); err != nil {
		return err
	}

	if magic[0] == 'P' && magic[1] == 'K' {
		st, err := f.Stat()
		if err != nil {
			return err
		}
		zr, err := zip.NewReader(f, st.Size())
		if err != nil {
			return err
		}
		return extractZip(zr, destDir)
	}

	if magic[0] == 0x1f && magic[1] == 0x8b {
		return extractTarGz(f, destDir)
	}

	return fmt.Errorf("unknown archive format (magic=%s)", strconv.QuoteToASCII(string(magic[:])))
}

func safeJoin(destDir, name string) (string, error) {
	name = filepath.Clean(name)
	name = strings.TrimPrefix(name, string(filepath.Separator))
	if name == "." || name == "" {
		return "", nil
	}
	if strings.HasPrefix(name, ".."+string(filepath.Separator)) || name == ".." {
		return "", fmt.Errorf("invalid path: %q", name)
	}
	target := filepath.Join(destDir, name)
	if rel, err := filepath.Rel(destDir, target); err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("invalid path: %q", name)
	}
	return target, nil
}

func extractZip(zr *zip.Reader, destDir string) error {
	var stripPrefix string
	for _, f := range zr.File {
		if stripPrefix == "" {
			parts := strings.SplitN(f.Name, "/", 2)
			if len(parts) > 0 {
				stripPrefix = parts[0] + "/"
			}
		}

		name := strings.TrimPrefix(f.Name, stripPrefix)
		if name == "" {
			continue
		}

		name = filepath.FromSlash(name)
		target, err := safeJoin(destDir, name)
		if err != nil {
			return err
		}
		if target == "" {
			continue
		}

		if strings.HasSuffix(f.Name, "/") {
			if err := os.MkdirAll(target, 0755); err != nil {
				return err
			}
			continue
		}

		if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
			return err
		}
		rc, err := f.Open()
		if err != nil {
			return err
		}
		out, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0644)
		if err != nil {
			_ = rc.Close()
			return err
		}
		if _, err := io.Copy(out, rc); err != nil {
			_ = out.Close()
			_ = rc.Close()
			return err
		}
		_ = out.Close()
		_ = rc.Close()
	}
	return nil
}

func extractTarGz(r io.Reader, destDir string) error {
	gz, err := gzip.NewReader(r)
	if err != nil {
		return err
	}
	defer func() { _ = gz.Close() }()

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
		name = filepath.FromSlash(name)
		target, err := safeJoin(destDir, name)
		if err != nil {
			return err
		}
		if target == "" {
			continue
		}

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
				_ = f.Close()
				return err
			}
			_ = f.Close()
		case tar.TypeSymlink:
			if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
				return err
			}
			_ = os.Remove(target)
			if err := os.Symlink(hdr.Linkname, target); err != nil {
				return err
			}
		}
	}
	return nil
}
