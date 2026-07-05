//! Rendering of `View`s. While the shared screen holds the terminal, views are
//! inserted above its viewport so output and widgets stay ordered; otherwise
//! they print to stdout — dimmed on a terminal, bare when piped or redirected
//! (so `| grep` and `> file` keep working). Long tables hand off to the pager.

use std::io::{self, IsTerminal};

use anyhow::Result;
use ratatui::crossterm::style::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use super::interactive;
use super::screen;
use super::view::View;

pub fn show(view: View) -> Result<()> {
    if stdout_is_tty() && screen::active() {
        return show_on_screen(view);
    }
    match view {
        View::Line(text) => println!("{text}"),
        View::Note(text) => note(&text),
        View::Detail(rows) => detail(&rows),
        View::Table { headers, rows, .. } => print_table(&headers, &rows),
    }
    Ok(())
}

/// The screen is holding the terminal: insert above its viewport instead of
/// printing at the cursor (which sits inside the viewport). Long tables page.
fn show_on_screen(view: View) -> Result<()> {
    if let View::Table {
        title,
        headers,
        rows,
    } = &view
    {
        let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
        if interactive::browse(title, &header_refs, rows)? {
            return Ok(());
        }
    }
    screen::insert(view_lines(view))
}

/// A view as styled lines for the scrollback (multi-line text is split — a
/// buffer line cannot hold a newline).
fn view_lines(view: View) -> Vec<Line<'static>> {
    let dim = Style::default().fg(Color::DarkGray);
    match view {
        View::Line(text) => text.split('\n').map(|l| Line::raw(l.to_string())).collect(),
        View::Note(text) => text
            .split('\n')
            .map(|l| Line::styled(l.to_string(), dim))
            .collect(),
        View::Detail(rows) => {
            let width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            rows.into_iter()
                .map(|(key, value)| {
                    Line::from(vec![
                        Span::styled(format!("{key:width$}"), dim),
                        Span::raw(format!("  {value}")),
                    ])
                })
                .collect()
        }
        View::Table { headers, rows, .. } => {
            let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
            let widths = column_widths(&header_refs, &rows);
            let mut lines = vec![Line::styled(render_row(&header_refs, &widths), dim)];
            lines.extend(rows.iter().map(|row| Line::raw(render_row(row, &widths))));
            lines
        }
    }
}

fn stdout_is_tty() -> bool {
    io::stdout().is_terminal()
}

fn note(text: &str) {
    if stdout_is_tty() {
        println!("{}", text.dark_grey());
    } else {
        println!("{text}");
    }
}

fn detail(rows: &[(String, String)]) {
    let width = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (key, value) in rows {
        let label = format!("{key:width$}");
        if stdout_is_tty() {
            println!("{}  {value}", label.dark_grey());
        } else {
            println!("{label}  {value}");
        }
    }
}

/// Print a left-aligned table (the plain path).
fn print_table(headers: &[String], rows: &[Vec<String>]) {
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let widths = column_widths(&header_refs, rows);
    let header = render_row(&header_refs, &widths);
    if stdout_is_tty() {
        println!("{}", header.dark_grey());
    } else {
        println!("{header}");
    }
    for row in rows {
        println!("{}", render_row(row, &widths));
    }
}

/// The width each column must be padded to: the widest of its header and cells.
pub fn column_widths(headers: &[&str], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }
    widths
}

/// Render one row's cells left-aligned to `widths`, joined by two spaces.
pub fn render_row<S: AsRef<str>>(cells: &[S], widths: &[usize]) -> String {
    let line: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                "{:width$}",
                c.as_ref(),
                width = widths.get(i).copied().unwrap_or(0)
            )
        })
        .collect();
    line.join("  ").trim_end().to_string()
}

/// Render a byte count in human units.
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
