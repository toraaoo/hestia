//! The terminal owner: the one ratatui inline viewport a CLI invocation may
//! hold. Every transient element — spinner, gauge, select, input, pager —
//! draws into this single viewport, and permanent output produced while it is
//! up is inserted *above* it (`insert_before`), scrolling into history in
//! order. A viewport per widget would scroll the terminal for each one and
//! leave the cursor drifting between them; one screen, created at the cursor
//! on first use and cleared exactly once at exit, keeps the whole invocation
//! in one place.
//!
//! The viewport starts as small as its first widget and only grows (a taller
//! widget clears and re-creates in place), so a spinner-only command reserves
//! one line, not the pager's fourteen.

use std::io::{self, IsTerminal, Stderr};
use std::sync::Mutex;

use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, MoveToColumn, Show};
use ratatui::crossterm::execute;
use ratatui::text::Line;
use ratatui::{Terminal, TerminalOptions, Viewport};

/// The tallest viewport any widget may request (the pager).
pub const MAX_HEIGHT: u16 = 14;

type Term = Terminal<CrosstermBackend<Stderr>>;

struct Screen {
    terminal: Term,
    height: u16,
}

static SCREEN: Mutex<Option<Screen>> = Mutex::new(None);

pub fn stderr_is_tty() -> bool {
    io::stderr().is_terminal()
}

/// Whether the screen exists — permanent terminal output must then route
/// through [`insert`] instead of printing at the (viewport-held) cursor.
pub fn active() -> bool {
    SCREEN.lock().unwrap().is_some()
}

/// Run `f` with the shared terminal at its current size (one line on first use).
pub fn with<T>(f: impl FnOnce(&mut Term) -> Result<T>) -> Result<T> {
    with_min(1, f)
}

/// Run `f` with the shared terminal, first growing the viewport to at least
/// `min` rows. Growth clears and re-creates in place, so the screen ratchets
/// up to the largest widget used and no further.
pub fn with_min<T>(min: u16, f: impl FnOnce(&mut Term) -> Result<T>) -> Result<T> {
    let min = min.clamp(1, MAX_HEIGHT);
    let mut guard = SCREEN.lock().unwrap();
    let grow = match guard.as_ref() {
        Some(screen) => screen.height < min,
        None => true,
    };
    if grow {
        if let Some(mut old) = guard.take() {
            let _ = old.terminal.clear();
        }
        let backend = CrosstermBackend::new(io::stderr());
        let terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(min),
            },
        )?;
        let _ = execute!(io::stderr(), Hide);
        *guard = Some(Screen {
            terminal,
            height: min,
        });
    }
    f(&mut guard.as_mut().expect("screen just ensured").terminal)
}

/// Blank the viewport (a finished widget's frame stays visible otherwise).
pub fn blank() {
    let _ = with(|terminal| {
        terminal.draw(|_| {})?;
        Ok(())
    });
}

/// Insert permanent lines above the viewport, in order, into scrollback.
pub fn insert(lines: Vec<Line<'static>>) -> Result<()> {
    with(|terminal| {
        terminal.insert_before(lines.len() as u16, |buf| {
            for (i, line) in lines.iter().enumerate() {
                buf.set_line(0, i as u16, line, buf.area.width);
            }
        })?;
        Ok(())
    })
}

/// Clear the viewport and hand the terminal back; called once at process end.
pub fn teardown() {
    let mut guard = SCREEN.lock().unwrap();
    if let Some(mut screen) = guard.take() {
        let _ = screen.terminal.clear();
        let _ = execute!(io::stderr(), MoveToColumn(0), Show);
    }
}
