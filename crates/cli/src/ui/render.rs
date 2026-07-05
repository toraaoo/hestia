//! Static rendering of `View`s to stdout: aligned tables and detail blocks,
//! dimmed on a terminal and plain when piped or redirected (so `| grep` and
//! `> file` keep working). Long tables hand off to the interactive pager.

use std::io::{self, IsTerminal};

use anyhow::Result;
use ratatui::crossterm::style::Stylize;

use super::interactive;
use super::view::View;

pub fn show(view: View) -> Result<()> {
    match view {
        View::Line(text) => println!("{text}"),
        View::Note(text) => note(&text),
        View::Detail(rows) => detail(&rows),
        View::Table {
            title,
            headers,
            rows,
        } => return table(&title, &headers, &rows),
    }
    Ok(())
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

fn table(title: &str, headers: &[String], rows: &[Vec<String>]) -> Result<()> {
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    if !interactive::browse(title, &header_refs, rows)? {
        print_table(&header_refs, rows);
    }
    Ok(())
}

/// Print a left-aligned table inline (the plain, non-paging path). Also the
/// fallback the pager degrades to when there is no terminal.
fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let widths = column_widths(headers, rows);
    let header = render_row(headers, &widths);
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
        .map(|(i, c)| format!("{:width$}", c.as_ref(), width = widths.get(i).copied().unwrap_or(0)))
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
