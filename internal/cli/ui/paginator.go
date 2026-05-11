package ui

import (
	"fmt"
	"os"
	"strings"

	"charm.land/lipgloss/v2"
	"golang.org/x/term"
)

func RunPaginator(items []string, perPage int) error {
	if !term.IsTerminal(int(os.Stdout.Fd())) {
		for _, item := range items {
			fmt.Println(item)
		}
		return nil
	}

	_, termH, err := term.GetSize(int(os.Stdout.Fd()))
	if err != nil {
		termH = perPage + 1
	}
	viewHeight := termH - 1

	if len(items) <= viewHeight {
		for _, item := range items {
			fmt.Println(item)
		}
		return nil
	}

	oldState, err := term.MakeRaw(int(os.Stdin.Fd()))
	if err != nil {
		for _, item := range items {
			fmt.Println(item)
		}
		return nil
	}
	defer term.Restore(int(os.Stdin.Fd()), oldState)

	// enter alternate screen + hide cursor
	fmt.Print("\033[?1049h\033[?25l")
	defer fmt.Print("\033[?1049l\033[?25h")

	offset := 0
	maxOffset := len(items) - viewHeight

	render := func() {
		var sb strings.Builder

		sb.WriteString("\033[H\033[2J")

		end := offset + viewHeight
		if end > len(items) {
			end = len(items)
		}

		for _, item := range items[offset:end] {
			sb.WriteString(item)
			sb.WriteString("\r\n")
		}

		percent := 0
		if maxOffset > 0 {
			percent = (offset * 100) / maxOffset
		}
		if offset >= maxOffset {
			percent = 100
		}

		status := fmt.Sprintf("lines %d-%d of %d (%d%%)", offset+1, end, len(items), percent)
		help := "j/k scroll  space/b page  g/G top/end  q quit"
		statusLine := lipgloss.NewStyle().Foreground(ColorMuted).Render(status + "  " + help)

		sb.WriteString(fmt.Sprintf("\033[%d;1H\033[2K", termH))
		sb.WriteString(statusLine)

		fmt.Print(sb.String())
	}

	render()

	buf := make([]byte, 3)
	for {
		n, err := os.Stdin.Read(buf)
		if err != nil {
			break
		}

		input := string(buf[:n])

		switch {
		case input == "q" || input == "\x03":
			return nil
		case input == "j" || input == "\x1b[B":
			if offset < maxOffset {
				offset++
			}
		case input == "k" || input == "\x1b[A":
			if offset > 0 {
				offset--
			}
		case input == " " || input == "\x1b[6~":
			offset += viewHeight
			if offset > maxOffset {
				offset = maxOffset
			}
		case input == "b" || input == "\x1b[5~":
			offset -= viewHeight
			if offset < 0 {
				offset = 0
			}
		case input == "g":
			offset = 0
		case input == "G":
			offset = maxOffset
		}
		render()
	}

	return nil
}
