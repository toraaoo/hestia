//! Overlay geometry: a centered rectangle for a modal, and a bordered panel
//! drawn into it. Screens compose these to float a picker or a checklist over
//! their body.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Block;
use ratatui::Frame;

/// A rectangle centered in `area`, sized to the given percentages of it.
pub fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);
    let [_, mid, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(mid);
    mid
}

/// Draw a titled, dark-bordered panel filling `area` and return its inner
/// content area. The caller is responsible for clearing underneath first.
pub fn bordered(frame: &mut Frame, area: Rect, title: &str) -> Rect {
    let block = Block::bordered()
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title.to_string());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}
