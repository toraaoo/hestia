package rcon

import (
	"fmt"

	"github.com/gorcon/rcon"
)

type Client struct {
	conn *rcon.Conn
}

func Dial(addr, password string) (*Client, error) {
	conn, err := rcon.Dial(addr, password)
	if err != nil {
		return nil, fmt.Errorf("rcon dial: %w", err)
	}
	return &Client{conn: conn}, nil
}

func (c *Client) Execute(cmd string) (string, error) {
	return c.conn.Execute(cmd)
}

func (c *Client) Close() error {
	return c.conn.Close()
}
