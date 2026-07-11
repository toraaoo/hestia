//! One tray per endpoint. The daemon spawns the tray on every serve without
//! checking for an existing one; a duplicate finds the lock taken and exits
//! before putting a second icon in the tray. The lock is keyed by the
//! endpoint the tray watches (`HESTIA_SOCK` included), so a dev daemon's tray
//! and the session's tray coexist instead of one silently controlling the
//! other's daemon. The lock file lives in the transport runtime dir
//! (ephemeral, per-user) and is held for the process lifetime — the OS
//! releases it at exit, crashes included.

use std::fs::File;
use std::path::Path;

pub struct Lock(#[allow(dead_code)] File);

pub fn acquire() -> Option<Lock> {
    let dir = ipc::endpoint::runtime_dir();
    std::fs::create_dir_all(&dir).ok()?;
    open_exclusive(&dir.join(lock_name())).map(Lock)
}

fn lock_name() -> String {
    let endpoint = ipc::endpoint::default_endpoint();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in endpoint.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("tray-{hash:016x}.lock")
}

#[cfg(unix)]
fn open_exclusive(path: &Path) -> Option<File> {
    use std::os::unix::io::AsRawFd;

    let file = File::create(path).ok()?;
    // SAFETY: flock on an fd we own; LOCK_NB makes the call non-blocking.
    let taken = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    taken.then_some(file)
}

#[cfg(windows)]
fn open_exclusive(path: &Path) -> Option<File> {
    use std::os::windows::fs::OpenOptionsExt;

    // share_mode(0) refuses every other open while this handle lives, so a
    // second tray fails here with a sharing violation.
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(path)
        .ok()
}
