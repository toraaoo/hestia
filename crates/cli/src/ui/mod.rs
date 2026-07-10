//! The CLI presentation layer. Commands never print directly — they build
//! `View`s and hand them here, and this module owns the terminal.
//!
//! The mode is decided once per invocation. **Interactive** (stdin and stderr
//! are terminals): every widget runs as a fullscreen session in the alternate
//! screen (`session`), built from shared `components`, and the terminal is
//! handed back intact when it resolves. Otherwise **plain**: text on stdout,
//! so `| grep` and `> file` keep working, and widgets degrade to arguments.
//! The future TUI (bare `hestia`) is one more `session::Screen` over the same
//! `View`s.

pub(crate) mod components;
mod console;
mod progress;
mod render;
mod screen;
pub(crate) mod session;
mod view;

use std::io::IsTerminal;
use std::sync::OnceLock;

use anyhow::{anyhow, bail, Result};

pub use console::ConsoleEvent;
pub use progress::{InstallReporter, ProvisionReporter, Spinner};
pub use view::View;

/// Whether this invocation can run widgets: stdin (keys) and stderr (drawing)
/// are both terminals. Decided once.
pub(crate) fn is_interactive() -> bool {
    static MODE: OnceLock<bool> = OnceLock::new();
    *MODE.get_or_init(|| std::io::stdin().is_terminal() && std::io::stderr().is_terminal())
}

/// Render a view: plainly to stdout, except tables too tall for the terminal,
/// which page in a fullscreen session when interactive.
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
    screen::teardown();
    session::run(session::prompt::SelectScreen::new(prompt, items), None)?
        .ok_or_else(|| anyhow!("selection cancelled"))
}

/// Prompt the user to check any number of `items`, returning their indices.
/// Requires an interactive terminal; errors otherwise so callers can ask for
/// arguments instead.
pub fn multi_select(prompt: &str, items: &[String]) -> Result<Vec<usize>> {
    if items.is_empty() {
        bail!("nothing to select");
    }
    if !is_interactive() {
        bail!("no interactive terminal; pass the choice as an argument");
    }
    screen::teardown();
    session::run(session::prompt::MultiSelectScreen::new(prompt, items), None)?
        .ok_or_else(|| anyhow!("selection cancelled"))
}

/// Ask for one line of input: typing edits, Enter accepts — empty takes
/// `default`, shown dim — Esc cancels with an error. Without an interactive
/// terminal the default is returned (the value is optional).
pub fn input(text: &str, default: &str) -> Result<String> {
    if !is_interactive() {
        return Ok(default.to_string());
    }
    screen::teardown();
    session::run(session::prompt::InputScreen::new(text, default), None)?
        .ok_or_else(|| anyhow!("input cancelled"))
}

/// Ask a yes/no question with labeled answers, returning `true` for `yes`.
/// Errors when the user cancels (Esc / q / Ctrl-C) or without an interactive
/// terminal, so callers can chain a hint naming the flag to pass instead.
pub fn confirm(prompt: &str, yes: &str, no: &str) -> Result<bool> {
    if !is_interactive() {
        bail!("no interactive terminal");
    }
    screen::teardown();
    session::run(session::prompt::ConfirmScreen::new(prompt, yes, no), None)?
        .ok_or_else(|| anyhow!("cancelled"))
}

/// Run the attach console: live output above an input line whose entries go
/// to `commands`. Blocking until detach or a `Closed` event, whose message it
/// returns. Requires an interactive terminal. The console owns the whole
/// terminal: it releases the shared viewport, runs fullscreen in the
/// alternate screen, and restores the original terminal on detach — anything
/// shown after prints plainly below it instead of into the (gone) viewport.
pub fn console(
    title: &str,
    backfill: Vec<String>,
    events: tokio::sync::mpsc::UnboundedReceiver<ConsoleEvent>,
    commands: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<Option<String>> {
    if !is_interactive() {
        bail!("no interactive terminal");
    }
    screen::teardown();
    console::run(title, backfill, events, commands)
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
