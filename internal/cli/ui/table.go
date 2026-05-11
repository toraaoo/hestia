package ui

import (
	"strings"

	"charm.land/lipgloss/v2"
)

func RenderTable(headers []string, rows [][]string, widths []int) string {
	colWidths := make([]int, len(headers))
	for i := range headers {
		if i < len(widths) {
			colWidths[i] = widths[i]
		} else {
			colWidths[i] = 12
		}
	}

	headerStyle := lipgloss.NewStyle().
		Bold(true).
		Foreground(ColorAccent).
		PaddingRight(2)

	cellStyle := lipgloss.NewStyle().PaddingRight(2)
	borderStyle := lipgloss.NewStyle().Foreground(ColorMuted)

	var sb strings.Builder

	for i, h := range headers {
		text := truncate(h, colWidths[i])
		sb.WriteString(headerStyle.Width(colWidths[i]).Render(text))
	}
	sb.WriteString("\n")

	totalWidth := 0
	for _, w := range colWidths {
		totalWidth += w + 2
	}
	sb.WriteString(borderStyle.Render(strings.Repeat("─", totalWidth)))
	sb.WriteString("\n")

	for _, row := range rows {
		for i := range headers {
			cell := ""
			if i < len(row) {
				cell = row[i]
			}
			text := truncate(cell, colWidths[i])
			sb.WriteString(cellStyle.Width(colWidths[i]).Render(text))
		}
		sb.WriteString("\n")
	}

	return sb.String()
}

func truncate(s string, max int) string {
	if len(s) <= max {
		return s
	}
	if max <= 3 {
		return s[:max]
	}
	return s[:max-3] + "..."
}
