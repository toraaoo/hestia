package api

import "net/http"

var shutdownCh chan struct{}

// Register wires all API routes onto mux.
// shutdown channel is closed when /shutdown is called.
func Register(mux *http.ServeMux, shutdown chan struct{}) {
	shutdownCh = shutdown
	mux.HandleFunc("GET /ping", ping)
	mux.HandleFunc("POST /shutdown", handleShutdown)
	mux.HandleFunc("GET /versions", handleVersions)
}

func ping(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
}

func handleShutdown(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	close(shutdownCh)
}
