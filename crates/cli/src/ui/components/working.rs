//! The in-session progress view: a centered gauge with the current phase
//! detail under it, fed by `ProvisionProgress` events while a job runs
//! inside a fullscreen session.

use client::proto::minecraft::ProvisionProgress;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

pub fn draw_working(frame: &mut Frame, label: &str, progress: Option<&ProvisionProgress>) {
    let [_, gauge_row, detail_row, _] = Layout::vertical([
        Constraint::Percentage(40),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(frame.area());
    let [_, gauge_area, _] = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ])
    .areas(gauge_row);
    let ratio = progress
        .filter(|p| p.total > 0)
        .map(|p| (p.current as f64 / p.total as f64).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    frame.render_widget(
        Gauge::default()
            .ratio(ratio)
            .gauge_style(Style::default().fg(Color::Cyan))
            .label(format!("{label} · {:.0}%", ratio * 100.0)),
        gauge_area,
    );
    let detail = progress
        .map(|p| {
            if p.items > 0 {
                format!("{}/{} · {}", p.item, p.items, p.detail)
            } else {
                p.detail.clone()
            }
        })
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(Line::styled(detail, Style::default().fg(Color::DarkGray))).centered(),
        detail_row,
    );
}
