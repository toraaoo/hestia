//! Launching the desktop shell from the tray. The binary sits beside the tray
//! (all front-ends live in the same dir) or in a `bin/` subdirectory. Spawned
//! detached — the desktop app outlives the tray. A second launch while it is
//! already running is absorbed by the shell's own single-instance handling
//! (GApplication re-focuses the existing window rather than opening another),
//! so the tray need not track it.

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn desktop_name() -> &'static str {
    if cfg!(windows) {
        "hestia-desktop.exe"
    } else {
        "hestia-desktop"
    }
}

fn find_desktop() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [
        dir.join(desktop_name()),
        dir.join("bin").join(desktop_name()),
    ]
    .into_iter()
    .find(|candidate| candidate.exists())
}

pub fn launch() {
    let Some(program) = find_desktop() else {
        tracing::warn!("desktop binary not found beside the tray; cannot open");
        return;
    };

    let mut cmd = Command::new(&program);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Detach into its own session — the desktop outlives the tray.
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
        Ok(child) => tracing::info!(pid = child.id(), "desktop launched"),
        Err(e) => tracing::warn!("cannot launch the desktop: {e}"),
    }
}
