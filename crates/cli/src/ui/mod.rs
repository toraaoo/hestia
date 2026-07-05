//! The CLI presentation layer. Commands never print directly — they build
//! `View`s and hand them here, and this module owns the terminal.
//!
//! The mode is decided once per invocation. **Interactive** (stdin and stderr
//! are terminals): the whole run shares one inline ratatui viewport
//! (`screen`) — every widget and progress line draws into it, and finished
//! output is inserted above it into scrollback, in order. Otherwise **plain**:
//! text on stdout, so `| grep` and `> file` keep working, and widgets degrade
//! to arguments. The future TUI (bare `hestia`) is a third driver over the
//! same `View`s.

mod interactive;
mod progress;
mod render;
mod screen;
mod view;

use std::io::IsTerminal;
use std::sync::OnceLock;

use anyhow::{bail, Result};

pub use progress::{InstallReporter, ProvisionReporter, Spinner};
pub use view::View;

/// Whether this invocation can run widgets: stdin (keys) and stderr (drawing)
/// are both terminals. Decided once.
pub(crate) fn is_interactive() -> bool {
    static MODE: OnceLock<bool> = OnceLock::new();
    *MODE.get_or_init(|| std::io::stdin().is_terminal() && std::io::stderr().is_terminal())
}

/// Render a view: into the session's scrollback while the screen holds the
/// terminal, plainly to stdout otherwise.
pub fn show(view: View) -> Result<()> {
    render::show(view)
}

/// Prompt the user to pick one of `items`, returning its index. Requires an
/// interactive terminal; errors otherwise so callers can ask for an argument.
pub fn select(prompt: &str, items: &[String]) -> Result<usize> {
    if items.is_empty() {
        bail!("nothing to select");
    }
    if !is_interactive() {
        bail!("no interactive terminal; pass the choice as an argument");
    }
    interactive::select(prompt, items)
}

/// Ask for one line of input: typing edits, Enter accepts — empty takes
/// `default`, shown dim — Esc cancels with an error. Without an interactive
/// terminal the default is returned (the value is optional).
pub fn input(text: &str, default: &str) -> Result<String> {
    if !is_interactive() {
        return Ok(default.to_string());
    }
    interactive::input(text, default)
}

/// Render a byte count in human units (KB, MB, …).
pub fn human_bytes(bytes: u64) -> String {
    render::human_bytes(bytes)
}

/// Return the terminal: clear the shared viewport and show the cursor. Called
/// once when the command finishes, before any final error print.
pub fn teardown() {
    screen::teardown();
}
