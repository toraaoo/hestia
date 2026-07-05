//! Terminal progress rendering for long-running commands. On an interactive
//! terminal an inline `ratatui` gauge (the FTXUI gauge's replacement) is drawn in
//! place; when stderr is redirected (a pipe, CI) it degrades to terse per-phase
//! lines so logs stay readable.

use std::io::{IsTerminal, Stderr};
use std::sync::Mutex;

use client::proto::java::{JavaInstallPhase, JavaInstallProgress};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Gauge;
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

use crate::output::human_bytes;

type InlineTerminal = Terminal<CrosstermBackend<Stderr>>;

/// Reports install progress, choosing an inline gauge or plain lines by whether
/// stderr is a terminal. Cheap to construct; safe to share across the async
/// progress callback (`Send + Sync`).
pub enum InstallReporter {
    Tty(Mutex<InlineTerminal>),
    Plain(Mutex<Option<JavaInstallPhase>>),
}

impl InstallReporter {
    pub fn new() -> Self {
        if std::io::stderr().is_terminal() {
            let backend = CrosstermBackend::new(std::io::stderr());
            let options = TerminalOptions {
                viewport: Viewport::Inline(1),
            };
            if let Ok(terminal) = Terminal::with_options(backend, options) {
                return InstallReporter::Tty(Mutex::new(terminal));
            }
        }
        InstallReporter::Plain(Mutex::new(None))
    }

    pub fn update(&self, progress: &JavaInstallProgress) {
        match self {
            InstallReporter::Tty(terminal) => {
                let mut terminal = terminal.lock().unwrap();
                let _ = terminal.draw(|frame| draw_gauge(frame, progress));
            }
            InstallReporter::Plain(last) => {
                let mut last = last.lock().unwrap();
                if *last != Some(progress.phase) {
                    *last = Some(progress.phase);
                    eprintln!("{}", phase_label(progress.phase));
                }
            }
        }
    }

    /// Clear the inline gauge so a following message prints on a clean line.
    pub fn finish(&self) {
        if let InstallReporter::Tty(terminal) = self {
            let _ = terminal.lock().unwrap().clear();
        }
    }
}

fn phase_label(phase: JavaInstallPhase) -> &'static str {
    match phase {
        JavaInstallPhase::Resolving => "resolving…",
        JavaInstallPhase::Downloading => "downloading…",
        JavaInstallPhase::Extracting => "extracting…",
    }
}

fn draw_gauge(frame: &mut Frame, progress: &JavaInstallProgress) {
    let (ratio, label) = match progress.phase {
        JavaInstallPhase::Resolving => (0.0, "resolving…".to_string()),
        JavaInstallPhase::Downloading => {
            let ratio = if progress.total > 0 {
                (progress.current as f64 / progress.total as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let total = if progress.total > 0 {
                human_bytes(progress.total)
            } else {
                "?".to_string()
            };
            (
                ratio,
                format!("downloading {} / {}", human_bytes(progress.current), total),
            )
        }
        JavaInstallPhase::Extracting => (1.0, "extracting…".to_string()),
    };

    let gauge = Gauge::default()
        .ratio(ratio)
        .label(label)
        .gauge_style(Style::default().fg(Color::Cyan));
    let area: Rect = frame.area();
    frame.render_widget(gauge, area);
}
