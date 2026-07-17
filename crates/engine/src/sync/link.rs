//! The cross-platform directory-link primitive behind linked sync folders: a
//! symlink on POSIX, a directory junction on Windows (junctions need no
//! privileges or developer mode, unlike Windows file/dir symlinks).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Create a directory link at `at` pointing to `store`. `at` must not exist;
/// `store` should already be a directory.
pub fn link_dir(store: &Path, at: &Path) -> Result<()> {
    platform::link_dir(store, at)
        .with_context(|| format!("cannot link {} -> {}", at.display(), store.display()))
}

/// The link target when `at` is a directory link (symlink/junction), `None`
/// for a regular directory, a file, or a missing path.
pub fn read_target(at: &Path) -> Option<PathBuf> {
    platform::read_target(at)
}

/// Whether `at` is a directory link pointing at exactly `store`.
pub fn is_linked_to(store: &Path, at: &Path) -> bool {
    read_target(at).is_some_and(|target| target == store)
}

/// Remove the directory link at `at`. Refuses (an error) when `at` is not a
/// link, so a real directory can never be deleted through this call.
pub fn unlink_dir(at: &Path) -> Result<()> {
    if read_target(at).is_none() {
        anyhow::bail!("{} is not a directory link", at.display());
    }
    platform::unlink_dir(at).with_context(|| format!("cannot unlink {}", at.display()))
}

/// Whether `path` is a directory that exists and holds no entries.
pub fn is_empty_dir(path: &Path) -> bool {
    match fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => false,
    }
}

#[cfg(unix)]
mod platform {
    use super::*;

    pub fn link_dir(store: &Path, at: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(store, at)
    }

    pub fn read_target(at: &Path) -> Option<PathBuf> {
        let meta = fs::symlink_metadata(at).ok()?;
        if !meta.file_type().is_symlink() {
            return None;
        }
        fs::read_link(at).ok()
    }

    pub fn unlink_dir(at: &Path) -> io::Result<()> {
        fs::remove_file(at)
    }
}

#[cfg(windows)]
mod platform {
    use super::*;

    pub fn link_dir(store: &Path, at: &Path) -> io::Result<()> {
        junction::create(store, at)
    }

    pub fn read_target(at: &Path) -> Option<PathBuf> {
        if !junction::exists(at).unwrap_or(false) {
            return None;
        }
        junction::get_target(at).ok()
    }

    pub fn unlink_dir(at: &Path) -> io::Result<()> {
        // A junction is a directory-shaped reparse point: deleting it is a
        // (non-recursive) directory remove, which detaches the reparse point
        // without touching the target's contents.
        fs::remove_dir(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("hestia-link-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn links_read_and_unlink() {
        let base = temp_dir("roundtrip");
        let store = base.join("store");
        let at = base.join("at");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join("marker.txt"), "x").unwrap();

        link_dir(&store, &at).unwrap();
        assert!(is_linked_to(&store, &at));
        assert_eq!(read_target(&at), Some(store.clone()));
        assert!(at.join("marker.txt").exists(), "reads through the link");

        unlink_dir(&at).unwrap();
        assert!(!at.exists());
        assert!(store.join("marker.txt").exists(), "the store is untouched");
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn read_target_is_none_for_real_paths() {
        let base = temp_dir("real");
        let dir = base.join("dir");
        fs::create_dir_all(&dir).unwrap();
        let file = base.join("file.txt");
        fs::write(&file, "x").unwrap();

        assert_eq!(read_target(&dir), None);
        assert_eq!(read_target(&file), None);
        assert_eq!(read_target(&base.join("missing")), None);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn unlink_refuses_a_real_directory() {
        let base = temp_dir("refuse");
        let dir = base.join("dir");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("keep.txt"), "x").unwrap();

        assert!(unlink_dir(&dir).is_err());
        assert!(dir.join("keep.txt").exists());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn empty_dir_detection() {
        let base = temp_dir("empty");
        let empty = base.join("empty");
        fs::create_dir_all(&empty).unwrap();
        let full = base.join("full");
        fs::create_dir_all(&full).unwrap();
        fs::write(full.join("f"), "x").unwrap();

        assert!(is_empty_dir(&empty));
        assert!(!is_empty_dir(&full));
        assert!(!is_empty_dir(&base.join("missing")));
        fs::remove_dir_all(&base).ok();
    }

    /// The one place a traversal bug destroys shared worlds: deleting a tree
    /// that contains a directory link must remove the link itself, never
    /// descend through it into the store.
    #[test]
    fn remove_dir_all_does_not_descend_through_a_link() {
        let base = temp_dir("noescape");
        let store = base.join("store");
        fs::create_dir_all(store.join("world")).unwrap();
        fs::write(store.join("world").join("level.dat"), "x").unwrap();
        let data = base.join("data");
        fs::create_dir_all(&data).unwrap();
        link_dir(&store, &data.join("saves")).unwrap();

        fs::remove_dir_all(&data).unwrap();
        assert!(!data.exists());
        assert!(
            store.join("world").join("level.dat").exists(),
            "the shared store must survive deleting a linked tree"
        );
        fs::remove_dir_all(&base).ok();
    }
}
