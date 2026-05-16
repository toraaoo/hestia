package httpc

import (
	"net/http"
	"time"
)

type Client struct {
	http      *http.Client
	userAgent string
}

func NewClient(httpClient *http.Client, userAgent string) *Client {
	if httpClient == nil {
		httpClient = &http.Client{Timeout: 30 * time.Second}
	}
	if userAgent == "" {
		userAgent = "hestia/1.0"
	}
	return &Client{http: httpClient, userAgent: userAgent}
}

func (c *Client) Get(url string) (*http.Response, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", c.userAgent)
	return c.http.Do(req)
}
