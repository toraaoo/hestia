package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"
)

type consoleRequest struct {
	Command string `json:"command"`
}

func handleLogs(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/logs")
	follow := r.URL.Query().Get("follow") == "true"
	lines := 100
	if n, err := strconv.Atoi(r.URL.Query().Get("lines")); err == nil && n > 0 {
		lines = n
	}

	if follow {
		handleLogStream(w, r, name, lines)
		return
	}

	logs, err := procManager.Logs(name, lines)
	if err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(logs)
}

func handleLogStream(w http.ResponseWriter, r *http.Request, name string, lines int) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		writeError(w, "streaming not supported", http.StatusInternalServerError)
		return
	}

	logs, _ := procManager.Logs(name, lines)
	ch, err := procManager.Subscribe(name)
	if err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}
	defer procManager.Unsubscribe(name, ch)

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")

	for _, line := range logs {
		fmt.Fprintf(w, "data: %s\n\n", strings.TrimSpace(line.Text))
	}
	flusher.Flush()

	for {
		select {
		case line, ok := <-ch:
			if !ok {
				return
			}
			fmt.Fprintf(w, "data: %s\n\n", strings.TrimSpace(line.Text))
			flusher.Flush()
		case <-r.Context().Done():
			return
		}
	}
}

func handleConsole(w http.ResponseWriter, r *http.Request) {
	name := extractServerName(r.URL.Path, "/console")

	var req consoleRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := procManager.SendCommand(name, req.Command); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}

	w.WriteHeader(http.StatusAccepted)
}
