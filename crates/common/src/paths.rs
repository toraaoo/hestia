//! Per-user data-directory resolution — the single source of truth for "where
//! Hestia's data lives", linked by the daemon (via the engine) and every client.

use std::fs;
use std::path::{Path, PathBuf};

fn env_path(name: &str) -> Option<PathBuf> {
    match std::env::var_os(name) {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => None,
    }
}

/// Debug-only: resolve `<workspace>/.hestia` from the running binary's location.
/// A dev binary sits at `<workspace>/target/<profile>/<bin>` (tests under
/// `target/<profile>/deps/`), so the workspace root is the parent of `target`.
/// Resolved at runtime so relocating the repo never needs a rebuild.
#[cfg(debug_assertions)]
fn dev_data_home() -> Option<PathBuf> {
    use std::ffi::OsStr;

    let exe = std::env::current_exe().ok()?;
    let target = exe
        .ancestors()
        .find(|p| p.file_name() == Some(OsStr::new("target")))?;
    Some(target.parent()?.join(".hestia"))
}

/// The platform default data directory. Debug builds anchor at `<workspace>/.hestia`
/// so development never touches the real per-user directory.
fn platform_data_home() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        if let Some(dir) = dev_data_home() {
            return dir;
        }
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = env_path("APPDATA") {
            return appdata.join("Hestia");
        }
        if let Some(profile) = env_path("USERPROFILE") {
            return profile.join("AppData").join("Roaming").join("Hestia");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = env_path("HOME") {
            return home.join(".hestia");
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// The fixed anchor directory: never redirected. Holds the persisted-home pointer
/// and is the default data directory when nothing else is configured.
pub fn anchor_dir() -> PathBuf {
    platform_data_home()
}

fn pointer_file() -> PathBuf {
    platform_data_home().join("home")
}

fn read_pointer() -> Option<PathBuf> {
    let contents = fs::read_to_string(pointer_file()).ok()?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

/// Resolve the data directory. Precedence: `override_dir` → `$HESTIA_HOME` → the
/// persisted-home pointer → the platform default.
pub fn data_home(override_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = override_dir {
        if !dir.as_os_str().is_empty() {
            return dir.to_path_buf();
        }
    }
    if let Some(env) = env_path("HESTIA_HOME") {
        return env;
    }
    if let Some(pointer) = read_pointer() {
        return pointer;
    }
    platform_data_home()
}

/// Persist `dir` as the default data directory for future runs. An empty path
/// removes the pointer, reverting to the platform default.
pub fn set_persisted_home(dir: &Path) -> std::io::Result<()> {
    let pointer = pointer_file();
    if dir.as_os_str().is_empty() {
        match fs::remove_file(&pointer) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        if let Some(parent) = pointer.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&pointer, format!("{}\n", dir.display()))
    }
}

/// The config file within the resolved data directory.
pub fn config_path(override_dir: Option<&Path>) -> PathBuf {
    data_home(override_dir).join("config")
}

/// The directory holding Hestia's own logs, within the resolved data directory.
pub fn log_dir(override_dir: Option<&Path>) -> PathBuf {
    data_home(override_dir).join("logs")
}
