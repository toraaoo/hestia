//! A scrollable text pane: wrap-aware vertical scrolling over a block of
//! styled lines, with an exact `n/m` position indicator. The pane owns its
//! scroll offset and remembers the area it last drew into, so key clamping and
//! mouse-wheel hit-testing work between frames. It draws no border of its own —
//! the caller frames it and hands in the inner content area.

use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

#[derive(Default)]
pub struct ScrollText {
    scroll: u16,
    max_scroll: u16,
    area: Rect,
}

impl ScrollText {
    /// Snap back to the top — call when the content being shown changes.
    pub fn reset(&mut self) {
        self.scroll = 0;
    }

    /// Scroll by a signed number of rows, clamped to the content.
    pub fn scroll_by(&mut self, delta: i32) {
        self.scroll = self
            .scroll
            .saturating_add_signed(delta as i16)
            .min(self.max_scroll);
    }

    /// Whether a cell falls within the pane as last drawn (for wheel routing).
    pub fn contains(&self, column: u16, row: u16) -> bool {
        self.area.contains(Position::new(column, row))
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, lines: Vec<Line>) {
        self.area = area;
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let total = paragraph.line_count(area.width) as u16;
        self.max_scroll = total.saturating_sub(area.height);
        self.scroll = self.scroll.min(self.max_scroll);
        frame.render_widget(paragraph.scroll((self.scroll, 0)), area);
        if self.max_scroll > 0 {
            let position = format!(" {}/{} ", self.scroll, self.max_scroll);
            let corner = Rect {
                x: area.right().saturating_sub(position.len() as u16),
                y: area.y,
                width: (position.len() as u16).min(area.width),
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(Line::styled(position, Style::default().fg(Color::DarkGray))),
                corner,
            );
        }
    }
}
