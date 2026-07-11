//! Terminal progress + spinner rendering for long-running commands. A
//! background thread rewrites one stderr line in place on a fixed tick — no
//! viewport, so the shell keeps its scrollback and a fullscreen session can
//! follow cleanly. When stderr is redirected (a pipe, CI) rendering is
//! skipped and callers that want a paper trail degrade to terse per-phase
//! lines.

use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use client::proto::java::{JavaInstallPhase, JavaInstallProgress};
use client::proto::minecraft::{ProvisionPhase, ProvisionProgress};
use ratatui::crossterm::cursor::{Hide, MoveToColumn, Show};
use ratatui::crossterm::execute;
use ratatui::crossterm::style::Stylize;
use ratatui::crossterm::terminal::{Clear, ClearType};

use super::render::human_bytes;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const TICK: Duration = Duration::from_millis(80);
const GAUGE_WIDTH: usize = 30;

/// What the animator paints on the next tick.
enum View {
    Spinner(String),
    Gauge {
        label: &'static str,
        ratio: f64,
        detail: String,
    },
}

/// A background renderer that rewrites the current `View` onto one stderr
/// line every tick until stopped. Created only when stderr is a terminal.
struct Animator {
    view: Arc<Mutex<View>>,
    stop: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Animator {
    fn start(initial: View) -> Option<Self> {
        if !io::stderr().is_terminal() {
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
    let mut err = io::stderr();
    let _ = execute!(err, Hide);
    let mut step = 0usize;
    while !stop.load(Ordering::Relaxed) {
        {
            let view = view.lock().unwrap();
            let spin = FRAMES[step % FRAMES.len()].cyan();
            let _ = execute!(err, MoveToColumn(0), Clear(ClearType::CurrentLine));
            let _ = write!(err, "{spin} {}", line(&view));
            let _ = err.flush();
        }
        step = step.wrapping_add(1);
        thread::sleep(TICK);
    }
    let _ = execute!(err, MoveToColumn(0), Clear(ClearType::CurrentLine), Show);
}

/// The line after the spinner frame: plain text (no escape codes — the line
/// is truncated by chars to the terminal width, and a sliced escape sequence
/// garbles the whole row).
fn line(view: &View) -> String {
    let text = match view {
        View::Spinner(label) => label.clone(),
        View::Gauge {
            label,
            ratio,
            detail,
        } => {
            let filled = ((ratio * GAUGE_WIDTH as f64).round() as usize).min(GAUGE_WIDTH);
            format!(
                "{label} {}{} {:>3.0}% · {detail}",
                "█".repeat(filled),
                "░".repeat(GAUGE_WIDTH - filled),
                ratio * 100.0
            )
        }
    };
    let width = ratatui::crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);
    match text.char_indices().nth(width.saturating_sub(3)) {
        Some((byte, _)) => text[..byte].to_string(),
        None => text,
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
                View::Gauge {
                    label: "downloading",
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

/// Reports provisioning progress (server create / instance launch): a live
/// gauge on a terminal, one line per phase when stderr is redirected. Byte
/// phases (java, jars) show sizes and throughput; count phases (libraries,
/// assets) show completed/total.
pub struct ProvisionReporter {
    animator: Option<Animator>,
    rate: Mutex<RateMeter>,
    last_phase: Mutex<Option<ProvisionPhase>>,
}

impl ProvisionReporter {
    pub fn new() -> Self {
        Self {
            animator: Animator::start(View::Spinner(
                provision_label(ProvisionPhase::Resolving).into(),
            )),
            rate: Mutex::new(RateMeter::default()),
            last_phase: Mutex::new(None),
        }
    }

    pub fn update(&self, progress: &ProvisionProgress) {
        let phase_changed = {
            let mut last = self.last_phase.lock().unwrap();
            let changed = *last != Some(progress.phase);
            *last = Some(progress.phase);
            changed
        };
        let Some(animator) = &self.animator else {
            if phase_changed {
                eprintln!("{}", provision_label(progress.phase));
            }
            return;
        };
        if phase_changed {
            *self.rate.lock().unwrap() = RateMeter::default();
        }
        let view = match progress.phase {
            ProvisionPhase::Java
            | ProvisionPhase::Server
            | ProvisionPhase::Client
            | ProvisionPhase::Content => {
                let rate = self.rate.lock().unwrap().observe(progress.current);
                View::Gauge {
                    label: gauge_label(progress.phase),
                    ratio: overall_ratio(progress),
                    detail: bytes_detail(progress, rate),
                }
            }
            ProvisionPhase::Libraries | ProvisionPhase::Assets | ProvisionPhase::Backup
                if progress.total > 0 =>
            {
                View::Gauge {
                    label: gauge_label(progress.phase),
                    ratio: count_ratio(progress.current, progress.total),
                    detail: format!(
                        "{} · {}/{}",
                        provision_noun(progress.phase),
                        progress.current,
                        progress.total
                    ),
                }
            }
            phase => View::Spinner(provision_label(phase).into()),
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

fn count_ratio(current: u64, total: u64) -> f64 {
    if total > 0 {
        (current as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The gauge ratio for a progress event. A multi-unit phase (`items > 0`)
/// fills monotonically across the whole batch — completed units plus the
/// current unit's byte fraction — so cached or instant units still advance
/// the bar instead of leaving it stuck at a per-file reset.
pub(crate) fn overall_ratio(progress: &ProvisionProgress) -> f64 {
    let unit = count_ratio(progress.current, progress.total);
    if progress.items > 0 {
        ((progress.item.saturating_sub(1) as f64 + unit) / progress.items as f64).clamp(0.0, 1.0)
    } else {
        unit
    }
}

fn bytes_detail(progress: &ProvisionProgress, rate: f64) -> String {
    let mut parts = Vec::new();
    if progress.items > 0 {
        parts.push(format!("{}/{}", progress.item, progress.items));
    }
    if progress.detail.is_empty() {
        parts.push(provision_noun(progress.phase).to_string());
    } else {
        parts.push(progress.detail.clone());
    }
    let total = if progress.total > 0 {
        human_bytes(progress.total)
    } else {
        "?".to_string()
    };
    parts.push(format!("{} / {total}", human_bytes(progress.current)));
    if rate > 0.0 {
        parts.push(format!("{}/s", human_bytes(rate as u64)));
    }
    parts.join(" · ")
}

/// The word before a phase's gauge (`backing up ███…`, not `downloading` for
/// everything).
fn gauge_label(phase: ProvisionPhase) -> &'static str {
    match phase {
        ProvisionPhase::Backup => "backing up",
        ProvisionPhase::Libraries => "libraries",
        ProvisionPhase::Assets => "assets",
        _ => "downloading",
    }
}

fn provision_label(phase: ProvisionPhase) -> &'static str {
    match phase {
        ProvisionPhase::Resolving => "resolving…",
        ProvisionPhase::Backup => "backing up…",
        ProvisionPhase::Java => "java runtime…",
        ProvisionPhase::Server => "server jar…",
        ProvisionPhase::Client => "client jar…",
        ProvisionPhase::Libraries => "libraries…",
        ProvisionPhase::Assets => "assets…",
        ProvisionPhase::Content => "downloading…",
    }
}

fn provision_noun(phase: ProvisionPhase) -> &'static str {
    match phase {
        ProvisionPhase::Resolving => "profile",
        ProvisionPhase::Backup => "files",
        ProvisionPhase::Java => "java",
        ProvisionPhase::Server => "server jar",
        ProvisionPhase::Client => "client jar",
        ProvisionPhase::Libraries => "libraries",
        ProvisionPhase::Assets => "assets",
        ProvisionPhase::Content => "content",
    }
}

/// Byte rate measured over fixed minimum windows. Progress events arrive over
/// the socket in bursts (one per chunk, several within microseconds), so a
/// per-event instantaneous rate is dominated by intra-burst spikes and wildly
/// overstates throughput; averaging each ≥`RATE_WINDOW` span weights fast and
/// stalled periods by their real duration.
#[derive(Default)]
struct RateMeter {
    window: Option<(Instant, u64)>,
    per_second: f64,
}

const RATE_WINDOW: Duration = Duration::from_millis(500);

impl RateMeter {
    fn observe(&mut self, current: u64) -> f64 {
        self.observe_at(Instant::now(), current)
    }

    fn observe_at(&mut self, now: Instant, current: u64) -> f64 {
        match self.window {
            Some((start, count)) if current >= count => {
                let elapsed = now.duration_since(start);
                if elapsed >= RATE_WINDOW {
                    let rate = (current - count) as f64 / elapsed.as_secs_f64();
                    self.per_second = if self.per_second == 0.0 {
                        rate
                    } else {
                        0.5 * self.per_second + 0.5 * rate
                    };
                    self.window = Some((now, current));
                }
            }
            // First observation, or the counter went backwards (a new file).
            _ => self.window = Some((now, current)),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_ignores_intra_burst_spikes() {
        let start = Instant::now();
        let mut meter = RateMeter::default();
        // 10 KB/s real throughput, delivered as micro-bursts of 5 events.
        let mut rate = 0.0;
        for second in 0..4u64 {
            for burst in 0..5u64 {
                let t = start + Duration::from_secs(second) + Duration::from_micros(burst * 50);
                rate = meter.observe_at(t, second * 10_000 + burst * 2_000);
            }
        }
        assert!(
            (5_000.0..20_000.0).contains(&rate),
            "expected ~10 KB/s, got {rate} B/s"
        );
    }

    #[test]
    fn rate_resets_when_the_counter_goes_backwards() {
        let start = Instant::now();
        let mut meter = RateMeter::default();
        meter.observe_at(start, 100_000);
        meter.observe_at(start + Duration::from_secs(1), 200_000);
        // A new file restarts the byte counter; no negative/huge sample.
        let rate = meter.observe_at(start + Duration::from_secs(2), 1_000);
        assert!((0.0..=100_000.0).contains(&rate));
    }
}
