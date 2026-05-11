package ui

import (
	"fmt"
	"io"
	"strings"
	"sync"

	"charm.land/lipgloss/v2"
	pb "github.com/toraaoo/hestia/internal/progress"
)

type MultiProgress struct {
	mu       sync.Mutex
	writer   io.Writer
	lines    map[pb.Category]*lineState
	order    []pb.Category
	rendered int
}

type lineState struct {
	label   string
	state   string
	current int64
	total   int64
}

func NewMultiProgress(w io.Writer) *MultiProgress {
	order := []pb.Category{pb.CategoryManifest, pb.CategoryJar, pb.CategoryJRE, pb.CategoryExtract}
	lines := map[pb.Category]*lineState{
		pb.CategoryManifest: {label: "Fetching manifest"},
		pb.CategoryJar:      {label: "Downloading jar"},
		pb.CategoryJRE:      {label: "Downloading JRE"},
		pb.CategoryExtract:  {label: "Extracting"},
	}
	for _, l := range lines {
		l.state = "waiting"
	}
	mp := &MultiProgress{writer: w, lines: lines, order: order}
	mp.render()
	return mp
}

func (m *MultiProgress) Handle(evt pb.Event) {
	m.mu.Lock()
	defer m.mu.Unlock()

	l := m.lines[evt.Category]
	if l == nil {
		return
	}

	switch evt.Type {
	case pb.EventStart:
		l.state = "active"
		l.total = evt.Total
	case pb.EventProgress:
		l.state = "active"
		l.current = evt.Current
		l.total = evt.Total
	case pb.EventComplete:
		l.state = "done"
	case pb.EventError:
		l.state = "error"
	}

	m.render()
}

func (m *MultiProgress) render() {
	if m.rendered > 0 {
		fmt.Fprintf(m.writer, "\033[%dA", m.rendered)
	}

	muted := lipgloss.NewStyle().Foreground(ColorMuted)
	done := lipgloss.NewStyle().Foreground(ColorSuccess)
	errStyle := lipgloss.NewStyle().Foreground(ColorError)

	for _, cat := range m.order {
		l := m.lines[cat]
		label := fmt.Sprintf("%-18s", l.label)

		var line string
		switch l.state {
		case "waiting":
			line = fmt.Sprintf("%s %s", label, muted.Render("waiting..."))
		case "done":
			line = fmt.Sprintf("%s %s", label, done.Render("done"))
		case "error":
			line = fmt.Sprintf("%s %s", label, errStyle.Render("failed"))
		case "active":
			bar := renderBar(l.current, l.total, 40)
			size := formatSize(l.current, l.total)
			line = fmt.Sprintf("%s %s %s", label, bar, size)
		}
		fmt.Fprintf(m.writer, "\033[2K%s\n", line)
	}
	m.rendered = len(m.order)
}

func renderBar(current, total int64, width int) string {
	if total <= 0 {
		return lipgloss.NewStyle().Foreground(ColorMuted).Render(strings.Repeat("░", width))
	}

	percent := float64(current) / float64(total)
	if percent > 1 {
		percent = 1
	}

	filled := int(percent * float64(width))
	empty := width - filled

	filledStyle := lipgloss.NewStyle().Foreground(ColorAccent)
	emptyStyle := lipgloss.NewStyle().Foreground(ColorMuted)

	return filledStyle.Render(strings.Repeat("█", filled)) +
		emptyStyle.Render(strings.Repeat("░", empty))
}

func formatSize(current, total int64) string {
	if total <= 0 {
		return humanize(current)
	}
	return fmt.Sprintf("%s / %s", humanize(current), humanize(total))
}

func humanize(b int64) string {
	const unit = 1024
	if b < unit {
		return fmt.Sprintf("%d B", b)
	}
	div, exp := int64(unit), 0
	for n := b / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %ciB", float64(b)/float64(div), "KMGTPE"[exp])
}

func (m *MultiProgress) SkipJRE() {
	m.mu.Lock()
	defer m.mu.Unlock()
	if l := m.lines[pb.CategoryJRE]; l != nil {
		l.state = "done"
		l.label = "JRE (cached)"
	}
	if l := m.lines[pb.CategoryExtract]; l != nil {
		l.state = "done"
		l.label = "Extract (skipped)"
	}
	m.render()
}

func (m *MultiProgress) Clear() {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.rendered > 0 {
		for range m.rendered {
			fmt.Fprintf(m.writer, "\033[2K\033[1A")
		}
		fmt.Fprintf(m.writer, "\033[2K")
		m.rendered = 0
	}
}
