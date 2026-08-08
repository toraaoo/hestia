//! The last good copy of a catalogue response, so a read that only needs to
//! know what versions exist still answers when the network is gone.
//!
//! Keyed by URL, kept under `meta/` with the rest of the derived files — every
//! entry is re-downloadable, and reclaiming the directory costs nothing but the
//! next fetch. Process-global for the same reason the client is: the catalogue
//! readers are provider impls that hold no aggregate reference.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use sha2::{Digest, Sha256};

fn root() -> &'static Mutex<Option<PathBuf>> {
    static ROOT: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    ROOT.get_or_init(|| Mutex::new(None))
}

/// Point the store at the running data home. Called by the engine at
/// construction and whenever the data home moves.
pub fn set_root(dir: PathBuf) {
    *root().lock().unwrap() = Some(dir);
}

fn entry_path(url: &str) -> Option<PathBuf> {
    let dir = root().lock().unwrap().clone()?;
    let digest = hex::encode(Sha256::digest(url.as_bytes()));
    Some(dir.join(format!("{digest}.json")))
}

pub fn save(url: &str, body: &str) {
    let Some(path) = entry_path(url) else {
        return;
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    // Written through a temp and renamed, so a reader never sees a half-file
    // and a crash mid-write leaves the previous good copy in place.
    let staging = path.with_extension("part");
    if std::fs::write(&staging, body).is_ok() && std::fs::rename(&staging, &path).is_err() {
        let _ = std::fs::remove_file(&staging);
    }
}

pub fn load(url: &str) -> Option<String> {
    std::fs::read_to_string(entry_path(url)?).ok()
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}
