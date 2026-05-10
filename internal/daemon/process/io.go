package process

import (
	"bufio"
	"io"
	"log/slog"
	"os"
	"path/filepath"
	"sync"
	"time"
)

type RingBuffer struct {
	lines []LogLine
	size  int
	pos   int
	mu    sync.RWMutex
}

type LogLine struct {
	Time time.Time `json:"time"`
	Text string    `json:"text"`
}

func NewRingBuffer(size int) *RingBuffer {
	return &RingBuffer{
		lines: make([]LogLine, size),
		size:  size,
	}
}

func (r *RingBuffer) Write(line string) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.lines[r.pos%r.size] = LogLine{Time: time.Now(), Text: line}
	r.pos++
}

func (r *RingBuffer) Last(n int) []LogLine {
	r.mu.RLock()
	defer r.mu.RUnlock()

	if n > r.size {
		n = r.size
	}
	if n > r.pos {
		n = r.pos
	}

	result := make([]LogLine, n)
	start := r.pos - n
	for i := 0; i < n; i++ {
		result[i] = r.lines[(start+i)%r.size]
	}
	return result
}

type LogWriter struct {
	ring *RingBuffer
	file *os.File
	subs []chan LogLine
	mu   sync.Mutex
}

func NewLogWriter(serverDir string, ring *RingBuffer) (*LogWriter, error) {
	logPath := filepath.Join(serverDir, "hestia.log")
	f, err := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		return nil, err
	}
	return &LogWriter{ring: ring, file: f}, nil
}

func (w *LogWriter) Write(p []byte) (int, error) {
	line := string(p)
	w.ring.Write(line)

	ll := LogLine{Time: time.Now(), Text: line}

	w.mu.Lock()
	if _, err := w.file.Write(p); err != nil {
		slog.Error("log file write failed", "err", err)
	}
	for _, ch := range w.subs {
		select {
		case ch <- ll:
		default:
		}
	}
	w.mu.Unlock()

	return len(p), nil
}

func (w *LogWriter) Subscribe() chan LogLine {
	ch := make(chan LogLine, 100)
	w.mu.Lock()
	w.subs = append(w.subs, ch)
	w.mu.Unlock()
	return ch
}

func (w *LogWriter) Unsubscribe(ch chan LogLine) {
	w.mu.Lock()
	defer w.mu.Unlock()
	for i, c := range w.subs {
		if c == ch {
			w.subs = append(w.subs[:i], w.subs[i+1:]...)
			close(ch)
			return
		}
	}
}

func (w *LogWriter) Close() error {
	w.mu.Lock()
	for _, ch := range w.subs {
		close(ch)
	}
	w.subs = nil
	w.mu.Unlock()
	return w.file.Close()
}

func ScanLines(r io.Reader, w *LogWriter) {
	scanner := bufio.NewScanner(r)
	for scanner.Scan() {
		w.Write(append(scanner.Bytes(), '\n'))
	}
}
