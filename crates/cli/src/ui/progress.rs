//! Terminal progress + spinner rendering for long-running commands. A background
//! thread owns a `ratatui` inline viewport (a single line, no alternate screen)
//! and redraws it on a fixed tick, so an indeterminate spinner keeps animating
//! and a download gauge stays smooth without the caller pumping frames. When
//! stderr is redirected (a pipe, CI) rendering is skipped and callers that want a
//! paper trail degrade to terse per-phase lines. Ported from the C++ CLI's
//! `Spinner`/`ProgressBar`.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use client::proto::java::{JavaInstallPhase, JavaInstallProgress};
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::{Frame, Terminal, TerminalOptions, Viewport};

use super::render::human_bytes;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const TICK: Duration = Duration::from_millis(80);
const GAUGE_WIDTH: u16 = 30;

/// What the animator paints on the next tick.
enum View {
    Spinner(String),
    Download { ratio: f64, detail: String },
}

/// A background renderer that owns the inline viewport and redraws the current
/// `View` every tick until stopped. Created only when stderr is a terminal.
struct Animator {
    view: Arc<Mutex<View>>,
    stop: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Animator {
    fn start(initial: View) -> Option<Self> {
        if !std::io::stderr().is_terminal() {
            return None;
        }
        let view = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        let (worker_view, worker_stop) = (view.clone(), stop.clone());
        let handle = thread::spawn(move || run(worker_view, worker_stop));
        Some(Self {
            view,
            stop,
            handle: Mutex::new(Some(handle)),
        })
    }

    fn set(&self, view: View) {
        *self.view.lock().unwrap() = view;
    }

    fn stop(&self) {
        if let Some(handle) = self.handle.lock().unwrap().take() {
            self.stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
        }
    }
}

impl Drop for Animator {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(view: Arc<Mutex<View>>, stop: Arc<AtomicBool>) {
    let backend = CrosstermBackend::new(std::io::stderr());
    let options = TerminalOptions {
        viewport: Viewport::Inline(1),
    };
    let Ok(mut terminal) = Terminal::with_options(backend, options) else {
        return;
    };
    let _ = execute!(std::io::stderr(), Hide);
    let mut step = 0usize;
    while !stop.load(Ordering::Relaxed) {
        {
            let view = view.lock().unwrap();
            let _ = terminal.draw(|frame| draw(frame, &view, step));
        }
        step = step.wrapping_add(1);
        thread::sleep(TICK);
    }
    let _ = terminal.clear();
    let _ = execute!(std::io::stderr(), Show);
}

fn draw(frame: &mut Frame, view: &View, step: usize) {
    let area = frame.area();
    let spin = Span::styled(
        FRAMES[step % FRAMES.len()],
        Style::default().fg(Color::Cyan),
    );
    match view {
        View::Spinner(label) => {
            let line = Line::from(vec![spin, Span::raw(" "), Span::raw(label.as_str())]);
            frame.render_widget(Paragraph::new(line), area);
        }
        View::Download { ratio, detail } => {
            let cols = Layout::horizontal([
                Constraint::Length(12),
                Constraint::Length(GAUGE_WIDTH),
                Constraint::Min(0),
            ])
            .split(area);
            frame.render_widget(Paragraph::new(Line::from("downloading ")), cols[0]);
            let gauge = Gauge::default()
                .ratio(*ratio)
                .gauge_style(Style::default().fg(Color::Cyan))
                .label(format!("{:.0}%", ratio * 100.0));
            frame.render_widget(gauge, cols[1]);
            frame.render_widget(Paragraph::new(Line::from(format!(" {detail}"))), cols[2]);
        }
    }
}

/// An animated wait indicator held for the lifetime of a daemon round-trip:
/// `let _spinner = Spinner::start("…");` clears itself on drop. A no-op when
/// stderr is not a terminal.
pub struct Spinner {
    animator: Option<Animator>,
}

impl Spinner {
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            animator: Animator::start(View::Spinner(label.into())),
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some(animator) = &self.animator {
            animator.stop();
        }
    }
}

/// Reports java-install progress: a live gauge on a terminal, one line per phase
/// when stderr is redirected.
pub struct InstallReporter {
    animator: Option<Animator>,
    rate: Mutex<RateMeter>,
    last_phase: Mutex<Option<JavaInstallPhase>>,
}

impl InstallReporter {
    pub fn new() -> Self {
        Self {
            animator: Animator::start(View::Spinner(
                phase_label(JavaInstallPhase::Resolving).into(),
            )),
            rate: Mutex::new(RateMeter::default()),
            last_phase: Mutex::new(None),
        }
    }

    pub fn update(&self, progress: &JavaInstallProgress) {
        let Some(animator) = &self.animator else {
            let mut last = self.last_phase.lock().unwrap();
            if *last != Some(progress.phase) {
                *last = Some(progress.phase);
                eprintln!("{}", phase_label(progress.phase));
            }
            return;
        };
        let view = match progress.phase {
            JavaInstallPhase::Downloading => {
                let rate = self.rate.lock().unwrap().observe(progress.current);
                View::Download {
                    ratio: ratio(progress),
                    detail: download_detail(progress, rate),
                }
            }
            phase => View::Spinner(phase_label(phase).into()),
        };
        animator.set(view);
    }

    /// Clear the gauge so a following message prints on a clean line.
    pub fn finish(&self) {
        if let Some(animator) = &self.animator {
            animator.stop();
        }
    }
}

fn ratio(progress: &JavaInstallProgress) -> f64 {
    if progress.total > 0 {
        (progress.current as f64 / progress.total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn download_detail(progress: &JavaInstallProgress, rate: f64) -> String {
    let current = human_bytes(progress.current);
    let total = if progress.total > 0 {
        human_bytes(progress.total)
    } else {
        "?".to_string()
    };
    if rate > 0.0 {
        format!("{current} / {total} · {}/s", human_bytes(rate as u64))
    } else {
        format!("{current} / {total}")
    }
}

/// Exponential-moving-average byte rate, smoothing throughput between progress
/// callbacks so the reported speed does not jitter.
#[derive(Default)]
struct RateMeter {
    last: Option<(Instant, u64)>,
    per_second: f64,
}

impl RateMeter {
    fn observe(&mut self, current: u64) -> f64 {
        let now = Instant::now();
        if let Some((last_time, last_count)) = self.last {
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.0 && current >= last_count {
                let instant = (current - last_count) as f64 / elapsed;
                self.per_second = if self.per_second == 0.0 {
                    instant
                } else {
                    0.7 * self.per_second + 0.3 * instant
                };
            }
        }
        self.last = Some((now, current));
        self.per_second
    }
}

fn phase_label(phase: JavaInstallPhase) -> &'static str {
    match phase {
        JavaInstallPhase::Resolving => "resolving…",
        JavaInstallPhase::Downloading => "downloading…",
        JavaInstallPhase::Extracting => "extracting…",
    }
}
