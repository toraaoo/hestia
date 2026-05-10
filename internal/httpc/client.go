package httpc

import (
	"net/http"
	"time"
)

var (
	Default  = &http.Client{Timeout: 30 * time.Second}
	Download = &http.Client{Timeout: 10 * time.Minute}
)

func Get(url string) (*http.Response, error) {
	return doGet(Default, url)
}

func GetDownload(url string) (*http.Response, error) {
	return doGet(Download, url)
}

func doGet(c *http.Client, url string) (*http.Response, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", "hestia/1.0")
	return c.Do(req)
}
