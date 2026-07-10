//! The fullscreen table pager: sticky header, scrollbar, key and wheel
//! scrolling, for tables too tall to print whole.

use std::convert::Infallible;

use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::Frame;

use super::{is_cancel, Flow, Screen};
use crate::ui::render::column_widths;

const WHEEL: isize = 3;

pub struct PagerScreen {
    title: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    constraints: Vec<Constraint>,
    state: TableState,
    page: isize,
}

impl PagerScreen {
    pub fn new(title: &str, headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
        let constraints = column_widths(&header_refs, &rows)
            .into_iter()
            .map(|w| Constraint::Length(w as u16))
            .collect();
        let mut state = TableState::default();
        state.select(Some(0));
        PagerScreen {
            title: title.to_string(),
            headers,
            rows,
            constraints,
            state,
            page: 10,
        }
    }

    fn move_row(&mut self, delta: isize) {
        let current = self.state.selected().unwrap_or(0) as isize;
        let last = self.rows.len() as isize - 1;
        self.state
            .select(Some((current + delta).clamp(0, last) as usize));
    }
}

impl Screen for PagerScreen {
    type Event = Infallible;
    type Outcome = ();

    fn wants_mouse(&self) -> bool {
        true
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.page = area.height.saturating_sub(3).max(1) as isize;
        let dim = Style::default().fg(Color::DarkGray);
        let selected = self.state.selected().unwrap_or(0);

        let header = Row::new(self.headers.iter().map(|h| Cell::from(h.as_str())))
            .style(Style::default().add_modifier(Modifier::BOLD));
        let body = self
            .rows
            .iter()
            .map(|r| Row::new(r.iter().cloned().map(Cell::from)));
        let footer = format!(
            " {}/{}  ·  j/k scroll · q quit ",
            selected + 1,
            self.rows.len()
        );

        let table = Table::new(body, self.constraints.clone())
            .header(header)
            .row_highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
            .block(
                Block::bordered()
                    .border_style(dim)
                    .title(self.title.clone())
                    .title_bottom(Line::from(footer).right_aligned()),
            );
        frame.render_stateful_widget(table, area, &mut self.state);

        let mut scrollbar = ScrollbarState::new(self.rows.len()).position(selected);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                horizontal: 0,
                vertical: 1,
            }),
            &mut scrollbar,
        );
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.move_row(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_row(-1),
            KeyCode::PageDown | KeyCode::Char(' ' | 'f') => self.move_row(self.page),
            KeyCode::PageUp | KeyCode::Char('b') => self.move_row(-self.page),
            KeyCode::Home | KeyCode::Char('g') => self.state.select(Some(0)),
            KeyCode::End | KeyCode::Char('G') => {
                self.state.select(Some(self.rows.len().saturating_sub(1)))
            }
            KeyCode::Enter => return Flow::Done(()),
            _ if is_cancel(&key) => return Flow::Done(()),
            _ => {}
        }
        Flow::Continue
    }

    fn on_mouse(&mut self, mouse: MouseEvent) -> Flow<Self::Outcome> {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.move_row(-WHEEL),
            MouseEventKind::ScrollDown => self.move_row(WHEEL),
            _ => {}
        }
        Flow::Continue
    }
}
