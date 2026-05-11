package client

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"time"

	"github.com/toraaoo/hestia/internal/progress"
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
	defer func() { _ = resp.Body.Close() }()
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

func (c *Client) DoRaw(_ context.Context, req *http.Request) (*http.Response, error) {
	resp, err := c.http.Do(req)
	if err != nil {
		return nil, fmt.Errorf("daemon unreachable: %w", err)
	}
	if resp.StatusCode >= 400 {
		_ = resp.Body.Close()
		return nil, fmt.Errorf("daemon error: %s", resp.Status)
	}
	return resp, nil
}

// Shared API types

type ServerInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Jar     string `json:"jar"`
	Port    int    `json:"port"`
	State   string `json:"state"`
	PID     int    `json:"pid,omitempty"`
}

type CreateRequest struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Memory  string `json:"memory,omitempty"`
	Port    int    `json:"port,omitempty"`
	Jar     string `json:"jar,omitempty"`

	// RCON
	RCONEnabled  *bool  `json:"rcon_enabled,omitempty"`
	RCONPassword string `json:"rcon_password,omitempty"`
	RCONPort     int    `json:"rcon_port,omitempty"`

	// World
	WorldName  string `json:"world_name,omitempty"`
	Seed       string `json:"seed,omitempty"`
	Gamemode   string `json:"gamemode,omitempty"`
	Difficulty string `json:"difficulty,omitempty"`
	MaxPlayers int    `json:"max_players,omitempty"`
	MOTD       string `json:"motd,omitempty"`
}

type LogLine struct {
	Time time.Time `json:"time"`
	Text string    `json:"text"`
}

type UpgradeRequest struct {
	Version  string `json:"version"`
	NoBackup bool   `json:"no_backup,omitempty"`
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

func (c *Client) CreateServerWithProgress(ctx context.Context, req CreateRequest, handler func(progress.Event)) (map[string]any, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, err
	}

	httpReq, err := http.NewRequestWithContext(ctx, "POST", "http://hestiad/servers", bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Accept", "text/event-stream")
	httpReq.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("daemon unreachable: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	return c.readSSE(resp.Body, handler)
}

func (c *Client) readSSE(r io.Reader, handler func(progress.Event)) (map[string]any, error) {
	scanner := bufio.NewScanner(r)
	for scanner.Scan() {
		line := scanner.Text()
		if !strings.HasPrefix(line, "data: ") {
			continue
		}
		data := strings.TrimPrefix(line, "data: ")

		var msg map[string]any
		if err := json.Unmarshal([]byte(data), &msg); err != nil {
			continue
		}

		if done, ok := msg["done"].(bool); ok && done {
			if result, ok := msg["result"].(map[string]any); ok {
				return result, nil
			}
			return nil, nil
		}

		var evt progress.Event
		if err := json.Unmarshal([]byte(data), &evt); err == nil {
			if evt.Type == progress.EventError && evt.Error != "" {
				return nil, fmt.Errorf("%s", evt.Error)
			}
			handler(evt)
		}
	}
	return nil, scanner.Err()
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

func (c *Client) UpgradeServer(ctx context.Context, name string, req UpgradeRequest) (map[string]any, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, err
	}
	var resp map[string]any
	return resp, c.Do(ctx, "POST", "/servers/"+name+"/upgrade", bytes.NewReader(body), &resp)
}

func (c *Client) UpgradeServerWithProgress(ctx context.Context, name string, req UpgradeRequest, handler func(progress.Event)) (map[string]any, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, err
	}

	httpReq, err := http.NewRequestWithContext(ctx, "POST", "http://hestiad/servers/"+name+"/upgrade", bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	httpReq.Header.Set("Accept", "text/event-stream")
	httpReq.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(httpReq)
	if err != nil {
		return nil, fmt.Errorf("daemon unreachable: %w", err)
	}
	defer func() { _ = resp.Body.Close() }()

	return c.readSSE(resp.Body, handler)
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

type BackupInfo struct {
	Name      string    `json:"name"`
	Path      string    `json:"path"`
	Type      string    `json:"type"`
	Size      int64     `json:"size"`
	CreatedAt time.Time `json:"created_at"`
	WorldName string    `json:"world_name,omitempty"`
	Version   string    `json:"version,omitempty"`
}

type BackupRequest struct {
	Type  string `json:"type,omitempty"`
	Force bool   `json:"force,omitempty"`
}

type PruneRequest struct {
	KeepLast   int `json:"keep_last,omitempty"`
	KeepDays   int `json:"keep_days,omitempty"`
	MinBackups int `json:"min_backups,omitempty"`
}

type PruneResult struct {
	Deleted int      `json:"deleted"`
	Names   []string `json:"names"`
}

func (c *Client) CreateBackup(ctx context.Context, serverName string, req BackupRequest) (*BackupInfo, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, err
	}
	var info BackupInfo
	return &info, c.Do(ctx, "POST", "/servers/"+serverName+"/backup", bytes.NewReader(body), &info)
}

func (c *Client) ListBackups(ctx context.Context, serverName string) ([]BackupInfo, error) {
	var backups []BackupInfo
	return backups, c.Do(ctx, "GET", "/servers/"+serverName+"/backups", nil, &backups)
}

func (c *Client) RestoreBackup(ctx context.Context, serverName, backupName string) (map[string]any, error) {
	var result map[string]any
	return result, c.Do(ctx, "POST", "/servers/"+serverName+"/backups/"+backupName+"/restore", nil, &result)
}

func (c *Client) DeleteBackup(ctx context.Context, serverName, backupName string) error {
	return c.Do(ctx, "DELETE", "/servers/"+serverName+"/backups/"+backupName, nil, nil)
}

func (c *Client) PruneBackups(ctx context.Context, serverName string, req PruneRequest) (*PruneResult, error) {
	body, err := json.Marshal(req)
	if err != nil {
		return nil, err
	}
	var result PruneResult
	return &result, c.Do(ctx, "POST", "/servers/"+serverName+"/backups/prune", bytes.NewReader(body), &result)
}
