package api

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/toraaoo/hestia/internal/progress"
)

type SSEWriter struct {
	w       http.ResponseWriter
	flusher http.Flusher
}

func NewSSEWriter(w http.ResponseWriter) (*SSEWriter, error) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		return nil, fmt.Errorf("streaming not supported")
	}

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	flusher.Flush()

	return &SSEWriter{w: w, flusher: flusher}, nil
}

func (s *SSEWriter) WriteEvent(evt progress.Event) error {
	data, err := json.Marshal(evt)
	if err != nil {
		return err
	}
	_, err = fmt.Fprintf(s.w, "data: %s\n\n", data)
	if err != nil {
		return err
	}
	s.flusher.Flush()
	return nil
}

func (s *SSEWriter) WriteError(errMsg string) error {
	evt := progress.Event{
		Type:  progress.EventError,
		Error: errMsg,
	}
	return s.WriteEvent(evt)
}

func (s *SSEWriter) WriteDone(result any) error {
	data, _ := json.Marshal(map[string]any{"done": true, "result": result})
	fmt.Fprintf(s.w, "data: %s\n\n", data)
	s.flusher.Flush()
	return nil
}
