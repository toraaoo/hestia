package download

import (
	"encoding/hex"
	"fmt"
	"hash"
	"io"
	"net/http"
	"os"
	"time"

	"crypto/sha1"
)

var client = &http.Client{Timeout: 10 * time.Minute}

// Options configures a download.
type Options struct {
	SHA1     string // expected hex checksum; skipped if empty
	Progress func(downloaded, total int64)
	Retries  int // default 3
}

// File downloads url to destPath atomically (temp file + rename).
// Retries on transient errors with exponential backoff.
func File(url, destPath string, opts Options) error {
	if opts.Retries == 0 {
		opts.Retries = 3
	}

	var lastErr error
	for attempt := range opts.Retries {
		if attempt > 0 {
			time.Sleep(time.Duration(attempt) * 2 * time.Second)
		}
		if err := tryDownload(url, destPath, opts); err != nil {
			lastErr = err
			continue
		}
		return nil
	}
	return lastErr
}

func tryDownload(url, destPath string, opts Options) (retErr error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("User-Agent", "hestia/1.0")

	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("fetch: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("http %s", resp.Status)
	}

	tmpPath := destPath + ".tmp"
	f, err := os.Create(tmpPath)
	if err != nil {
		return fmt.Errorf("create temp: %w", err)
	}
	defer func() {
		if retErr != nil {
			f.Close()
			os.Remove(tmpPath)
		}
	}()

	var h hash.Hash
	if opts.SHA1 != "" {
		h = sha1.New()
	}

	var r io.Reader = resp.Body
	if opts.Progress != nil {
		r = &progressReader{r: resp.Body, total: resp.ContentLength, fn: opts.Progress}
	}

	var dst io.Writer = f
	if h != nil {
		dst = io.MultiWriter(f, h)
	}

	if _, err := io.Copy(dst, r); err != nil {
		return fmt.Errorf("write: %w", err)
	}
	if err := f.Close(); err != nil {
		return fmt.Errorf("close: %w", err)
	}

	if h != nil {
		got := hex.EncodeToString(h.Sum(nil))
		if got != opts.SHA1 {
			os.Remove(tmpPath)
			return fmt.Errorf("sha1 mismatch: got %s, want %s", got, opts.SHA1)
		}
	}

	return os.Rename(tmpPath, destPath)
}

type progressReader struct {
	r          io.Reader
	total      int64
	downloaded int64
	fn         func(downloaded, total int64)
}

func (pr *progressReader) Read(p []byte) (int, error) {
	n, err := pr.r.Read(p)
	pr.downloaded += int64(n)
	pr.fn(pr.downloaded, pr.total)
	return n, err
}
