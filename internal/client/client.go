package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"time"
)

// Client sends requests to the hestiad daemon over a Unix socket.
type Client struct {
	http *http.Client
}

func New(sockPath string) *Client {
	transport := &http.Transport{
		DialContext: func(ctx context.Context, _, _ string) (net.Conn, error) {
			return dial(ctx, sockPath)
		},
	}
	return &Client{http: &http.Client{Transport: transport}}
}

func (c *Client) Do(ctx context.Context, method, path string, body io.Reader, dest any) error {
	req, err := http.NewRequestWithContext(ctx, method, "http://hestiad"+path, body)
	if err != nil {
		return err
	}
	resp, err := c.http.Do(req)
	if err != nil {
		return fmt.Errorf("daemon unreachable: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode >= 400 {
		var e struct{ Error string }
		if json.NewDecoder(resp.Body).Decode(&e) == nil && e.Error != "" {
			return fmt.Errorf("%s", e.Error)
		}
		return fmt.Errorf("daemon error: %s", resp.Status)
	}
	if dest != nil {
		return json.NewDecoder(resp.Body).Decode(dest)
	}
	return nil
}

func (c *Client) DoRaw(ctx context.Context, req *http.Request) (*http.Response, error) {
	resp, err := c.http.Do(req)
	if err != nil {
		return nil, fmt.Errorf("daemon unreachable: %w", err)
	}
	if resp.StatusCode >= 400 {
		resp.Body.Close()
		return nil, fmt.Errorf("daemon error: %s", resp.Status)
	}
	return resp, nil
}

// Shared API types

type ServerInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Port    int    `json:"port"`
	State   string `json:"state"`
	PID     int    `json:"pid,omitempty"`
}

type CreateRequest struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Memory  string `json:"memory,omitempty"`
	Port    int    `json:"port,omitempty"`
}

type LogLine struct {
	Time time.Time `json:"time"`
	Text string    `json:"text"`
}

// Typed methods

func (c *Client) ListServers(ctx context.Context) ([]ServerInfo, error) {
	var servers []ServerInfo
	return servers, c.Do(ctx, "GET", "/servers", nil, &servers)
}

func (c *Client) GetServer(ctx context.Context, name string) (map[string]any, error) {
	var info map[string]any
	return info, c.Do(ctx, "GET", "/servers/"+name, nil, &info)
}

func (c *Client) CreateServer(ctx context.Context, req CreateRequest) (map[string]any, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, err
	}
	var resp map[string]any
	return resp, c.Do(ctx, "POST", "/servers", bytes.NewReader(body), &resp)
}

func (c *Client) StartServer(ctx context.Context, name string) error {
	return c.Do(ctx, "POST", "/servers/"+name+"/start", nil, nil)
}

func (c *Client) StopServer(ctx context.Context, name string) error {
	return c.Do(ctx, "POST", "/servers/"+name+"/stop", nil, nil)
}

func (c *Client) RestartServer(ctx context.Context, name string) error {
	return c.Do(ctx, "POST", "/servers/"+name+"/restart", nil, nil)
}

func (c *Client) DeleteServer(ctx context.Context, name string) error {
	return c.Do(ctx, "DELETE", "/servers/"+name, nil, nil)
}

func (c *Client) GetLogs(ctx context.Context, name string, lines int) ([]LogLine, error) {
	var logs []LogLine
	path := fmt.Sprintf("/servers/%s/logs?lines=%d", name, lines)
	return logs, c.Do(ctx, "GET", path, nil, &logs)
}

func (c *Client) GetConfig(ctx context.Context, name string) (map[string]any, error) {
	var cfg map[string]any
	return cfg, c.Do(ctx, "GET", "/servers/"+name+"/config", nil, &cfg)
}

func (c *Client) UpdateConfig(ctx context.Context, name string, updates map[string]any) error {
	body, err := json.Marshal(updates)
	if err != nil {
		return err
	}
	return c.Do(ctx, "PUT", "/servers/"+name+"/config", bytes.NewReader(body), nil)
}

func (c *Client) SendConsoleCommand(ctx context.Context, name, command string) error {
	body, err := json.Marshal(map[string]string{"command": command})
	if err != nil {
		return err
	}
	return c.Do(ctx, "POST", "/servers/"+name+"/console", bytes.NewReader(body), nil)
}
