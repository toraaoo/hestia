//! Interactive widgets — a single-select prompt, a one-line text input, and a
//! scrollable pager — implemented as modal key-loops drawing into the shared
//! screen. Each widget requests only the viewport height it needs; the mode
//! gate (an interactive terminal) is the facade's job.

use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{self, IsTerminal};

use anyhow::{anyhow, Result};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Cell, List, ListItem, ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState,
};
use ratatui::Frame;

use super::render::column_widths;
use super::screen;

/// The select shows at most this many items at once (it scrolls past them).
const MAX_SELECT_ROWS: u16 = 12;

/// Drive one widget: draw through the shared screen at `height` rows, feed it
/// key presses until it resolves, then blank the viewport for whatever runs
/// next. Raw mode spans the loop; the screen lock is never held while waiting
/// for a key.
fn run_widget<T>(
    height: u16,
    mut draw: impl FnMut(&mut Frame),
    mut on_key: impl FnMut(KeyEvent) -> Option<Result<T>>,
) -> Result<T> {
    enable_raw_mode()?;
    let result = loop {
        let drawn = screen::with_min(height, |terminal| {
            terminal.draw(&mut draw)?;
            Ok(())
        });
        if let Err(e) = drawn {
            break Err(e);
        }
        let key = match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => key,
            Ok(_) => continue,
            Err(e) => break Err(e.into()),
        };
        if let Some(outcome) = on_key(key) {
            break outcome;
        }
    };
    let _ = disable_raw_mode();
    screen::blank();
    result
}

fn is_cancel(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

/// Present `items` under `prompt` and return the chosen index. Errors when the
/// user cancels (Esc / q / Ctrl-C).
pub fn select(prompt: &str, items: &[String]) -> Result<usize> {
    let state = RefCell::new(ListState::default());
    state.borrow_mut().select(Some(0));
    let height = (items.len() as u16).min(MAX_SELECT_ROWS) + 1;
    run_widget(
        height,
        |frame| draw_select(frame, prompt, items, &mut state.borrow_mut()),
        |key| match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                step(&mut state.borrow_mut(), items.len(), -1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                step(&mut state.borrow_mut(), items.len(), 1);
                None
            }
            KeyCode::Enter => Some(Ok(state.borrow().selected().unwrap_or(0))),
            _ if is_cancel(&key) => Some(Err(anyhow!("selection cancelled"))),
            _ => None,
        },
    )
}

/// Present `items` under `prompt` with checkboxes and return the checked
/// indices (Space toggles, Enter confirms — with nothing checked, Enter takes
/// the highlighted row). Errors when the user cancels.
pub fn multi_select(prompt: &str, items: &[String]) -> Result<Vec<usize>> {
    let state = RefCell::new(ListState::default());
    state.borrow_mut().select(Some(0));
    let checked = RefCell::new(HashSet::<usize>::new());
    let height = (items.len() as u16).min(MAX_SELECT_ROWS) + 2;
    run_widget(
        height,
        |frame| {
            draw_multi_select(
                frame,
                prompt,
                items,
                &checked.borrow(),
                &mut state.borrow_mut(),
            )
        },
        |key| match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                step(&mut state.borrow_mut(), items.len(), -1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                step(&mut state.borrow_mut(), items.len(), 1);
                None
            }
            KeyCode::Char(' ') => {
                let current = state.borrow().selected().unwrap_or(0);
                let mut checked = checked.borrow_mut();
                if !checked.remove(&current) {
                    checked.insert(current);
                }
                None
            }
            KeyCode::Enter => {
                let checked = checked.borrow();
                let mut chosen: Vec<usize> = if checked.is_empty() {
                    vec![state.borrow().selected().unwrap_or(0)]
                } else {
                    checked.iter().copied().collect()
                };
                chosen.sort_unstable();
                Some(Ok(chosen))
            }
            _ if is_cancel(&key) => Some(Err(anyhow!("selection cancelled"))),
            _ => None,
        },
    )
}

/// A single-line text prompt: type to edit, Enter accepts (empty takes
/// `default`, shown dim), Esc cancels.
pub fn input(prompt: &str, default: &str) -> Result<String> {
    let typed = RefCell::new(String::new());
    run_widget(
        1,
        |frame| draw_input(frame, prompt, &typed.borrow(), default),
        |key| match key.code {
            KeyCode::Enter => {
                let typed = typed.borrow();
                let value = typed.trim();
                Some(Ok(if value.is_empty() {
                    default.to_string()
                } else {
                    value.to_string()
                }))
            }
            KeyCode::Backspace => {
                typed.borrow_mut().pop();
                None
            }
            KeyCode::Esc => Some(Err(anyhow!("input cancelled"))),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Err(anyhow!("input cancelled")))
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                typed.borrow_mut().push(c);
                None
            }
            _ => None,
        },
    )
}

/// Page through a table interactively (sticky header, scrollbar, scroll, quit).
/// Returns `true` when it handled display. Returns `false` — so the caller
/// falls back to inserting/printing it whole — when keys are unavailable or
/// the table already fits in the tallest viewport.
pub fn browse(title: &str, headers: &[&str], rows: &[Vec<String>]) -> Result<bool> {
    if rows.is_empty() || !io::stdin().is_terminal() || !screen::stderr_is_tty() {
        return Ok(false);
    }
    if rows.len() as u16 + 3 <= screen::MAX_HEIGHT {
        return Ok(false);
    }

    let constraints: Vec<Constraint> = column_widths(headers, rows)
        .into_iter()
        .map(|w| Constraint::Length(w as u16))
        .collect();
    let page = screen::MAX_HEIGHT.saturating_sub(3).max(1) as isize;

    let state = RefCell::new(TableState::default());
    state.borrow_mut().select(Some(0));
    run_widget(
        screen::MAX_HEIGHT,
        |frame| {
            draw_pager(
                frame,
                title,
                headers,
                rows,
                &constraints,
                &mut state.borrow_mut(),
            )
        },
        |key| {
            let len = rows.len();
            let mut state = state.borrow_mut();
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => move_row(&mut state, len, 1),
                KeyCode::Up | KeyCode::Char('k') => move_row(&mut state, len, -1),
                KeyCode::PageDown | KeyCode::Char(' ' | 'f') => move_row(&mut state, len, page),
                KeyCode::PageUp | KeyCode::Char('b') => move_row(&mut state, len, -page),
                KeyCode::Home | KeyCode::Char('g') => state.select(Some(0)),
                KeyCode::End | KeyCode::Char('G') => state.select(Some(len - 1)),
                KeyCode::Enter => return Some(Ok(true)),
                _ if is_cancel(&key) => return Some(Ok(true)),
                _ => {}
            }
            None
        },
    )
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

fn draw_multi_select(
    frame: &mut Frame,
    prompt: &str,
    items: &[String],
    checked: &HashSet<usize>,
    state: &mut ListState,
) {
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());
    frame.render_widget(
        Paragraph::new(Line::from(prompt)).style(Style::default().fg(Color::Cyan)),
        layout[0],
    );
    let list = List::new(items.iter().enumerate().map(|(i, item)| {
        let mark = if checked.contains(&i) { "[x] " } else { "[ ] " };
        ListItem::new(format!("{mark}{item}"))
    }))
    .highlight_symbol("> ")
    .highlight_style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(list, layout[1], state);
    frame.render_widget(
        Paragraph::new(Line::from("space toggles · enter confirms"))
            .style(Style::default().fg(Color::DarkGray)),
        layout[2],
    );
}

fn draw_input(frame: &mut Frame, prompt: &str, typed: &str, default: &str) {
    let mut spans = vec![
        Span::styled(format!("{prompt}: "), Style::default().fg(Color::Cyan)),
        Span::raw(typed.to_string()),
        Span::styled("▏", Style::default().fg(Color::Cyan)),
    ];
    if typed.is_empty() {
        spans.push(Span::styled(
            default.to_string(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(frame.area());
    frame.render_widget(Paragraph::new(Line::from(spans)), rows[0]);
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
