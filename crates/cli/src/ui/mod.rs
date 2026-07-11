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
pub(crate) mod markdown;
mod progress;
mod render;
pub(crate) mod session;
mod view;

use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

use anyhow::{anyhow, bail, Result};

pub use components::PickerItem;
pub use progress::{DownloadReporter, InstallReporter, ProvisionReporter, Spinner};
pub use session::console::ConsoleEvent;
pub use view::View;

/// Whether this invocation can run widgets: stdin (keys) and stderr (drawing)
/// are both terminals. Decided once.
pub(crate) fn is_interactive() -> bool {
    static MODE: OnceLock<bool> = OnceLock::new();
    *MODE.get_or_init(|| std::io::stdin().is_terminal() && std::io::stderr().is_terminal())
}

/// Whether a full session may own the invocation: [`is_interactive`] *and*
/// stdout is a terminal. A piped stdout means the caller is consuming
/// results, so result-producing flows (browse, wizards, log followers,
/// attach-on-start) degrade to their plain forms — while argument prompts,
/// which draw on stderr, may still ask.
pub(crate) fn interactive_output() -> bool {
    static MODE: OnceLock<bool> = OnceLock::new();
    *MODE.get_or_init(|| is_interactive() && std::io::stdout().is_terminal())
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
    session::run(session::prompt::InputScreen::new(text, default), None)?
        .ok_or_else(|| anyhow!("input cancelled"))
}

/// Prompt through a searchable picker (type to filter, Tab widens the pool to
/// unstable entries), returning the chosen item's index into `items`. Requires
/// an interactive terminal; errors otherwise so callers can ask for an
/// argument.
pub fn pick(prompt: &str, items: Vec<PickerItem>) -> Result<usize> {
    if items.is_empty() {
        bail!("nothing to select");
    }
    if !is_interactive() {
        bail!("no interactive terminal; pass the choice as an argument");
    }
    session::run(session::prompt::PickerScreen::new(prompt, items), None)?
        .ok_or_else(|| anyhow!("selection cancelled"))
}

/// Ask for one line inline on the current screen: the prompt goes to stderr
/// and the reply is read from stdin — no session, so the exchange stays in
/// the scrollback. Errors without an interactive terminal.
pub fn prompt(text: &str) -> Result<String> {
    if !is_interactive() {
        bail!("no interactive terminal");
    }
    eprint!("{text}: ");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Ask a yes/no question inline: `[y/N]` appended to the prompt, defaulting
/// to no. Errors without an interactive terminal.
pub fn prompt_confirm(text: &str) -> Result<bool> {
    let answer = prompt(&format!("{text} [y/N]"))?;
    Ok(matches!(answer.to_lowercase().as_str(), "y" | "yes"))
}

/// Pick one of `items` inline: a numbered list on stderr, the number read
/// from stdin (an empty answer cancels). Errors without an interactive
/// terminal.
pub fn prompt_select(text: &str, items: &[String]) -> Result<usize> {
    if items.is_empty() {
        bail!("nothing to select");
    }
    if !is_interactive() {
        bail!("no interactive terminal; pass the choice as an argument");
    }
    eprintln!("{text}:");
    for (index, item) in items.iter().enumerate() {
        eprintln!("  {}. {item}", index + 1);
    }
    loop {
        let answer = prompt(&format!("1-{}", items.len()))?;
        if answer.is_empty() {
            bail!("selection cancelled");
        }
        match answer.parse::<usize>() {
            Ok(n) if (1..=items.len()).contains(&n) => return Ok(n - 1),
            _ => eprintln!("enter a number between 1 and {}", items.len()),
        }
    }
}

/// Ask a yes/no question with labeled answers, returning `true` for `yes`.
/// Errors when the user cancels (Esc / q / Ctrl-C) or without an interactive
/// terminal, so callers can chain a hint naming the flag to pass instead.
pub fn confirm(prompt: &str, yes: &str, no: &str) -> Result<bool> {
    if !is_interactive() {
        bail!("no interactive terminal");
    }
    session::run(session::prompt::ConfirmScreen::new(prompt, yes, no), None)?
        .ok_or_else(|| anyhow!("cancelled"))
}

/// Run the attach console: live output above an input line whose entries go
/// to `commands`. Blocking until detach or a `Closed` event, whose message it
/// returns. Requires an interactive terminal.
pub fn console(
    title: &str,
    backfill: Vec<String>,
    events: tokio::sync::mpsc::UnboundedReceiver<ConsoleEvent>,
    commands: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<Option<String>> {
    if !is_interactive() {
        bail!("no interactive terminal");
    }
    session::run(
        session::console::ConsoleScreen::new(title, backfill, commands),
        Some(events),
    )
}

/// Run a read-only log session: fullscreen live output, scrollable, no input
/// line. Blocking until detach (returns `None`, the workload keeps running)
/// or a `Closed` event, whose message it returns. Requires an interactive
/// terminal.
pub fn log_session(
    title: &str,
    backfill: Vec<String>,
    events: tokio::sync::mpsc::UnboundedReceiver<ConsoleEvent>,
) -> Result<Option<String>> {
    if !is_interactive() {
        bail!("no interactive terminal");
    }
    session::run(session::logs::LogScreen::new(title, backfill), Some(events))
}

/// Render a byte count in human units (KB, MB, …).
pub fn human_bytes(bytes: u64) -> String {
    render::human_bytes(bytes)
}
