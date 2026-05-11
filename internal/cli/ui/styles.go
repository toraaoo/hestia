package ui

import (
	"fmt"
	"io"
	"time"

	"charm.land/lipgloss/v2"
)

var (
	ColorSuccess = lipgloss.Color("#22c55e")
	ColorError   = lipgloss.Color("#ef4444")
	ColorMuted   = lipgloss.Color("#6b7280")
	ColorAccent  = lipgloss.Color("#3b82f6")
)

var (
	StateRunning = lipgloss.NewStyle().Foreground(ColorSuccess)
	StateStopped = lipgloss.NewStyle().Foreground(ColorMuted)
	StateError   = lipgloss.NewStyle().Foreground(ColorError)
	Prompt       = lipgloss.NewStyle().Foreground(ColorAccent).Bold(true)
)

func StateStyle(state string) lipgloss.Style {
	switch state {
	case "running":
		return StateRunning
	case "stopped":
		return StateStopped
	case "error":
		return StateError
	default:
		return lipgloss.NewStyle()
	}
}

type Spinner struct {
	frames []string
	idx    int
	msg    string
	w      io.Writer
	stop   chan struct{}
	done   chan struct{}
}

func NewSpinner(w io.Writer, msg string) *Spinner {
	return &Spinner{
		frames: []string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"},
		msg:    msg,
		w:      w,
		stop:   make(chan struct{}),
		done:   make(chan struct{}),
	}
}

func (s *Spinner) Start() {
	go func() {
		ticker := time.NewTicker(80 * time.Millisecond)
		defer ticker.Stop()
		defer close(s.done)
		for {
			select {
			case <-s.stop:
				_, _ = fmt.Fprintf(s.w, "\r\033[2K")
				return
			case <-ticker.C:
				frame := lipgloss.NewStyle().Foreground(ColorAccent).Render(s.frames[s.idx])
				_, _ = fmt.Fprintf(s.w, "\r%s %s", frame, s.msg)
				s.idx = (s.idx + 1) % len(s.frames)
			}
		}
	}()
}

func (s *Spinner) Stop() {
	close(s.stop)
	<-s.done
}
