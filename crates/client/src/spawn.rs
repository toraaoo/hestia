//! Locating and auto-starting the daemon. `hestiad` sits beside the current
//! binary (all front-ends live in the same dir) or in a `bin/` subdirectory,
//! else it is resolved through PATH.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use ipc::Connection;

fn daemon_name() -> &'static str {
    if cfg!(windows) {
        "hestiad.exe"
    } else {
        "hestiad"
    }
}

fn find_daemon() -> PathBuf {
    let exe = daemon_name();
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            for candidate in [dir.join(exe), dir.join("bin").join(exe)] {
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from(exe) // resolved through PATH
}

/// Start the daemon detached from this process's session, so it outlives the
/// front-end that spawned it.
pub fn spawn_daemon() -> std::io::Result<()> {
    let program = find_daemon();
    tracing::debug!(program = %program.display(), "spawning the daemon");
    let mut cmd = Command::new(program);
    cmd.arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Detach from the frontend's session — the daemon outlives it.
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

    cmd.spawn().map(|_| ())
}

/// Poll briefly for a freshly-spawned daemon's endpoint to appear.
pub async fn connect_with_retry(endpoint: &std::path::Path) -> Option<Connection> {
    for attempt in 1..=60 {
        if let Ok(conn) = ipc::connect(endpoint).await {
            tracing::debug!(attempt, "connected to the spawned daemon");
            return Some(conn);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tracing::warn!(
        endpoint = %endpoint.display(),
        "spawned daemon never came up on its endpoint"
    );
    None
}
