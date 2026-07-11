//! RAII ownership of the terminal for a fullscreen session: raw mode plus the
//! alternate screen (and, opted in, mouse capture), released in reverse order
//! on drop. Drop runs on unwind too, so a panicking screen hands the shell
//! back sane before the error surfaces.

use std::io;

use anyhow::Result;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

pub struct TerminalGuard {
    mouse: bool,
}

impl TerminalGuard {
    pub fn acquire(mouse: bool) -> Result<Self> {
        enable_raw_mode()?;
        let entered = if mouse {
            execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)
        } else {
            execute!(io::stderr(), EnterAlternateScreen)
        };
        if let Err(e) = entered {
            let _ = disable_raw_mode();
            return Err(e.into());
        }
        // Console log lines (any crate's tracing warning) drawn over the
        // alternate screen corrupt it; the file sink keeps recording.
        common::logging::set_console_muted(true);
        Ok(TerminalGuard { mouse })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        common::logging::set_console_muted(false);
        let restored = if self.mouse {
            execute!(io::stderr(), DisableMouseCapture, LeaveAlternateScreen)
        } else {
            execute!(io::stderr(), LeaveAlternateScreen)
        };
        let _ = restored;
        let _ = disable_raw_mode();
    }
}
