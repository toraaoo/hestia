//! The CLI presentation layer. Commands never print directly — they build
//! `View`s and hand them here, and this module owns all output. On a terminal it
//! renders with ratatui (interactive select, scrollable pager, live progress);
//! piped or redirected it degrades to plain text so output stays scriptable. The
//! future TUI (bare `hestia`) will be a second driver over the same `View`s.

mod interactive;
mod progress;
mod render;
mod view;

use std::io::Write;

use anyhow::Result;

pub use progress::{InstallReporter, ProvisionReporter, Spinner};
pub use view::View;

/// Write an input prompt (no trailing newline) and flush, ahead of a stdin read.
pub fn prompt(text: &str) {
    print!("{text}");
    let _ = std::io::stdout().flush();
}

/// Ask for one line of input (an inline ratatui prompt): typing edits, Enter
/// accepts — empty takes `default`, shown dim — Esc cancels with an error.
/// Without an interactive terminal the default is returned (the value is
/// optional).
pub fn input(text: &str, default: &str) -> Result<String> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Ok(default.to_string());
    }
    interactive::input(text, default)
}

/// Render a static view: a line, a note, a key/value block, or a table (long
/// tables page interactively on a terminal).
pub fn show(view: View) -> Result<()> {
    render::show(view)
}

/// Prompt the user to pick one of `items`, returning its index. Requires an
/// interactive terminal; errors otherwise so callers can ask for an argument.
pub fn select(prompt: &str, items: &[String]) -> Result<usize> {
    interactive::select(prompt, items)
}

/// Render a byte count in human units (KB, MB, …).
pub fn human_bytes(bytes: u64) -> String {
    render::human_bytes(bytes)
}
