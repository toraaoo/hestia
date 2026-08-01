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

/// A `data/` directory inside the build's own layout, used when the data must
/// travel with the binaries instead of living in the per-user directory: the
/// portable archives, and every debug build.
///
/// The anchor is the layout's *root*, never the executable's own directory, so
/// that every binary of one build agrees on one home. Two directories are
/// stepped out of to find it:
///
/// - `bin/` — the installed and portable layout, where the support binaries sit
///   below the desktop shell;
/// - `deps/` — where cargo puts test binaries, which must resolve the same home
///   as the `target/<profile>/` binaries they exercise.
#[cfg(any(feature = "portable", debug_assertions))]
fn contained_data_home() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    contained_root(exe.parent()?).map(|root| root.join("data"))
}

/// The layout root holding `dir`, stepping out of a `bin/` or `deps/` directory.
#[cfg(any(feature = "portable", debug_assertions))]
fn contained_root(dir: &Path) -> Option<&Path> {
    use std::ffi::OsStr;

    match dir.file_name() {
        Some(name) if name == OsStr::new("bin") || name == OsStr::new("deps") => dir.parent(),
        _ => Some(dir),
    }
}

/// The platform default data directory. Portable and debug builds keep their
/// data inside the build's own layout, so neither ever touches the real
/// per-user directory.
fn platform_data_home() -> PathBuf {
    #[cfg(any(feature = "portable", debug_assertions))]
    {
        if let Some(dir) = contained_data_home() {
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
    data_home(override_dir).join("config.json")
}

/// The directory holding Hestia's own logs, within the resolved data directory.
pub fn log_dir(override_dir: Option<&Path>) -> PathBuf {
    data_home(override_dir).join("logs")
}

#[cfg(all(test, any(feature = "portable", debug_assertions)))]
mod tests {
    use super::*;

    #[test]
    fn every_binary_of_one_layout_agrees_on_the_root() {
        let root = Path::new("/opt/hestia");
        // The desktop shell sits at the root, its support binaries below it, and
        // cargo's test binaries a level down from the profile dir.
        for dir in [root, &root.join("bin"), &root.join("deps")] {
            assert_eq!(contained_root(dir), Some(root), "{}", dir.display());
        }
    }

    #[test]
    fn an_ordinary_directory_name_is_the_root_itself() {
        let dir = Path::new("/opt/hestia/binaries");
        assert_eq!(contained_root(dir), Some(dir));
    }
}
