//! Interactive terminal widgets over a `ratatui` inline viewport (no alternate
//! screen): a single-select prompt and a scrollable pager for long tables. Both
//! require an interactive terminal; callers degrade to an argument or a plain
//! dump when there is none.

use std::io::{self, IsTerminal};

use anyhow::{bail, Result};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Cell, List, ListItem, ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState,
};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

use super::render::column_widths;

type Stderr = Terminal<CrosstermBackend<io::Stderr>>;

const MAX_SELECT_ROWS: u16 = 12;
const MAX_PAGER_ROWS: u16 = 20;

/// Prompts draw on stderr and read stdin, so both must be a terminal.
fn can_prompt() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Run `body` against an inline viewport of `height` rows, restoring the terminal
/// afterwards regardless of outcome.
fn with_viewport<T>(height: u16, body: impl FnOnce(&mut Stderr) -> Result<T>) -> Result<T> {
    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?;
    enable_raw_mode()?;
    let _ = execute!(io::stderr(), Hide);
    let result = body(&mut terminal);
    let _ = disable_raw_mode();
    let _ = execute!(io::stderr(), Show);
    let _ = terminal.clear();
    result
}

/// Present `items` under `prompt` and return the chosen index. Errors if there is
/// no interactive terminal or the user cancels (Esc / q / Ctrl-C).
pub fn select(prompt: &str, items: &[String]) -> Result<usize> {
    if items.is_empty() {
        bail!("nothing to select");
    }
    if !can_prompt() {
        bail!("no interactive terminal; pass the choice as an argument");
    }

    let height = (items.len() as u16).min(MAX_SELECT_ROWS) + 1;
    with_viewport(height, |terminal| {
        let mut state = ListState::default();
        state.select(Some(0));
        loop {
            terminal.draw(|frame| draw_select(frame, prompt, items, &mut state))?;
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => step(&mut state, items.len(), -1),
                KeyCode::Down | KeyCode::Char('j') => step(&mut state, items.len(), 1),
                KeyCode::Enter => return Ok(state.selected().unwrap_or(0)),
                KeyCode::Esc | KeyCode::Char('q') => bail!("selection cancelled"),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    bail!("selection cancelled")
                }
                _ => {}
            }
        }
    })
}

/// Page through a table interactively (sticky header, scrollbar, scroll, quit).
/// Returns `true` when it handled display. Returns `false` — so the caller falls
/// back to a plain dump — when there is no terminal, output is being captured, or
/// the table already fits on screen (no point taking over the terminal for it).
pub fn browse(title: &str, headers: &[&str], rows: &[Vec<String>]) -> Result<bool> {
    if rows.is_empty() || !can_prompt() || !io::stdout().is_terminal() {
        return Ok(false);
    }
    let term_height = size().map(|(_, h)| h).unwrap_or(24);
    if rows.len() as u16 + 3 <= term_height {
        return Ok(false);
    }

    let constraints: Vec<Constraint> = column_widths(headers, rows)
        .into_iter()
        .map(|w| Constraint::Length(w as u16))
        .collect();
    let height = (rows.len() as u16 + 3).min(MAX_PAGER_ROWS);
    let page = height.saturating_sub(3).max(1) as isize;

    with_viewport(height, |terminal| {
        let mut state = TableState::default();
        state.select(Some(0));
        loop {
            terminal
                .draw(|frame| draw_pager(frame, title, headers, rows, &constraints, &mut state))?;
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let len = rows.len();
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => move_row(&mut state, len, 1),
                KeyCode::Up | KeyCode::Char('k') => move_row(&mut state, len, -1),
                KeyCode::PageDown | KeyCode::Char(' ' | 'f') => move_row(&mut state, len, page),
                KeyCode::PageUp | KeyCode::Char('b') => move_row(&mut state, len, -page),
                KeyCode::Home | KeyCode::Char('g') => state.select(Some(0)),
                KeyCode::End | KeyCode::Char('G') => state.select(Some(len - 1)),
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => return Ok(true),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(true)
                }
                _ => {}
            }
        }
    })
}

fn step(state: &mut ListState, len: usize, delta: isize) {
    let current = state.selected().unwrap_or(0) as isize;
    state.select(Some((current + delta).rem_euclid(len as isize) as usize));
}

fn move_row(state: &mut TableState, len: usize, delta: isize) {
    let current = state.selected().unwrap_or(0) as isize;
    state.select(Some((current + delta).clamp(0, len as isize - 1) as usize));
}

fn draw_select(frame: &mut Frame, prompt: &str, items: &[String], state: &mut ListState) {
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(frame.area());
    frame.render_widget(
        Paragraph::new(Line::from(prompt)).style(Style::default().fg(Color::Cyan)),
        layout[0],
    );
    let list = List::new(items.iter().map(|i| ListItem::new(i.as_str())))
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, layout[1], state);
}

fn draw_pager(
    frame: &mut Frame,
    title: &str,
    headers: &[&str],
    rows: &[Vec<String>],
    constraints: &[Constraint],
    state: &mut TableState,
) {
    let area = frame.area();
    let dim = Style::default().fg(Color::DarkGray);
    let selected = state.selected().unwrap_or(0);

    let header = Row::new(headers.iter().copied().map(Cell::from))
        .style(Style::default().add_modifier(Modifier::BOLD));
    let body = rows
        .iter()
        .map(|r| Row::new(r.iter().cloned().map(Cell::from)));
    let footer = format!(" {}/{}  ·  j/k scroll · q quit ", selected + 1, rows.len());

    let table = Table::new(body, constraints.to_vec())
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
                .title(title.to_string())
                .title_bottom(Line::from(footer).right_aligned()),
        );
    frame.render_stateful_widget(table, area, state);

    let mut scrollbar = ScrollbarState::new(rows.len()).position(selected);
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
