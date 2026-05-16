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

func (h *Handler) handleLogs(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	follow := r.URL.Query().Get("follow") == "true"
	lines := 100
	if n, err := strconv.Atoi(r.URL.Query().Get("lines")); err == nil && n > 0 {
		lines = n
	}

	if follow {
		h.handleLogStream(w, r, name, lines)
		return
	}

	logs, err := h.processes.Logs(name, lines)
	if err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(logs)
}

func (h *Handler) handleLogStream(w http.ResponseWriter, r *http.Request, name string, lines int) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		writeError(w, "streaming not supported", http.StatusInternalServerError)
		return
	}

	logs, _ := h.processes.Logs(name, lines)
	ch, err := h.processes.Subscribe(name)
	if err != nil {
		writeError(w, err.Error(), http.StatusNotFound)
		return
	}
	defer h.processes.Unsubscribe(name, ch)

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")

	for _, line := range logs {
		_, _ = fmt.Fprintf(w, "data: %s\n\n", strings.TrimSpace(line.Text))
	}
	flusher.Flush()

	for {
		select {
		case line, ok := <-ch:
			if !ok {
				return
			}
			_, _ = fmt.Fprintf(w, "data: %s\n\n", strings.TrimSpace(line.Text))
			flusher.Flush()
		case <-r.Context().Done():
			return
		}
	}
}

func (h *Handler) handleConsole(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")

	var req consoleRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, err.Error(), http.StatusBadRequest)
		return
	}

	if err := h.processes.SendCommand(name, req.Command); err != nil {
		writeError(w, err.Error(), http.StatusConflict)
		return
	}

	w.WriteHeader(http.StatusAccepted)
}
