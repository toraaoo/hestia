//! Spawning the tray helper: whenever hestiad serves the user session's
//! endpoint, the tray accompanies it. Best-effort — a headless session, a
//! missing binary, or an endpoint override (tests, side-by-side daemons)
//! simply means no tray. The tray itself enforces one instance per session,
//! so spawning on every serve needs no duplicate check here.

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
    if ipc::endpoint::is_overridden() {
        tracing::debug!("endpoint override active; not spawning the tray");
        return;
    }
    if !display_available() {
        tracing::debug!("no display; not spawning the tray");
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
