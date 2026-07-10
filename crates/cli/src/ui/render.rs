//! Rendering of `View`s: plain text on stdout — dimmed labels on a terminal,
//! bare when piped or redirected (so `| grep` and `> file` keep working).
//! Tables too tall for the terminal hand off to the fullscreen pager when the
//! invocation is interactive.

use std::io::{self, IsTerminal};

use anyhow::Result;
use ratatui::crossterm::style::Stylize;

use super::session;
use super::session::pager::PagerScreen;
use super::view::View;

pub fn show(view: View) -> Result<()> {
    if let View::Table {
        title,
        headers,
        rows,
    } = &view
    {
        if paged(title, headers, rows)? {
            return Ok(());
        }
    }
    match view {
        View::Line(text) => println!("{text}"),
        View::Note(text) => note(&text),
        View::Detail(rows) => detail(&rows),
        View::Table { headers, rows, .. } => print_table(&headers, &rows),
    }
    Ok(())
}

/// Page a table too tall for the terminal in a fullscreen session; `false`
/// hands it back for plain printing.
fn paged(title: &str, headers: &[String], rows: &[Vec<String>]) -> Result<bool> {
    if rows.is_empty() || !stdout_is_tty() || !super::is_interactive() {
        return Ok(false);
    }
    let height = ratatui::crossterm::terminal::size()
        .map(|(_, h)| h)
        .unwrap_or(0);
    if rows.len() as u16 + 3 <= height {
        return Ok(false);
    }
    session::run(
        PagerScreen::new(title, headers.to_vec(), rows.to_vec()),
        None,
    )?;
    Ok(true)
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
