//! Spawning the tray helper: whenever hestiad serves, the tray accompanies it
//! — a daemon on an overridden endpoint included, since the spawned tray
//! inherits `HESTIA_SOCK` and follows its daemon. Best-effort: a headless
//! session, a missing binary, or `HESTIA_NO_TRAY` (automated tests) simply
//! means no tray. The tray itself enforces one instance per endpoint, so
//! spawning on every serve needs no duplicate check here.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn tray_name() -> &'static str {
    if cfg!(windows) {
        "tray.exe"
    } else {
        "tray"
    }
}

fn find_tray() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join(tray_name()), dir.join("bin").join(tray_name())]
        .into_iter()
        .find(|candidate| candidate.exists())
}

fn suppressed() -> bool {
    std::env::var_os("HESTIA_NO_TRAY").is_some_and(|value| !value.is_empty() && value != "0")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn display_available() -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY"]
        .iter()
        .any(|var| std::env::var_os(var).is_some_and(|value| !value.is_empty()))
}

#[cfg(any(windows, target_os = "macos"))]
fn display_available() -> bool {
    true
}

pub fn spawn() {
    if suppressed() {
        tracing::debug!("HESTIA_NO_TRAY set; not spawning the tray");
        return;
    }
    if !display_available() {
        tracing::debug!("no interactive display; not spawning the tray");
        return;
    }
    let Some(program) = find_tray() else {
        tracing::debug!("tray binary not found beside hestiad; not spawning");
        return;
    };

    let mut cmd = Command::new(&program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Detach into its own session — the tray outlives the daemon by design.
        // SAFETY: setsid is async-signal-safe and valid in the forked child.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(DETACHED_PROCESS);
    }

    match cmd.spawn() {
        Ok(child) => tracing::info!(pid = child.id(), "tray spawned"),
        Err(e) => tracing::warn!("cannot spawn the tray: {e}"),
    }
}
