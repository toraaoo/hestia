//! Linked sync targets: a folder in an instance's `data/` is a directory link
//! into the store, so its content — worlds above all — is stored once and shared
//! live. Nothing here reconciles content; the link *is* the sharing.
//!
//! A folder that is missing, empty, or already linked into a hestia store
//! becomes a link. One holding the instance's own files is **adopted**, its
//! entries moving into the store. The automatic pass never overwrites: a name
//! the store already has stops the move until the user resolves the clash.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::sync::LinkState;
use proto::warning::NotSharedReason;

use super::files::copy_file;
use super::link;

/// Whether a folder target ended up linked into the store, and if not, which
/// arm of the guard refused it.
pub enum Linked {
    Yes,
    No(NotSharedReason),
}

/// What an adopt does with a name the store already has.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OnCollision {
    /// Refuse the whole target, naming the clashes; nothing moves.
    Refuse,
    /// Keep the store's copy and discard the instance's — only an instance
    /// deliberately rejoining sharing asks for this.
    KeepStore,
}

/// The apply pass for one folder target: nothing to do when already linked; a
/// stale hestia-store link (the data home moved) is relinked; a missing or empty
/// directory becomes a link; a directory holding the instance's own files is
/// adopted. A foreign link and a folder whose names the store already has are
/// never touched, and are reported rather than logged — the target the user
/// asked to share is not shared.
pub fn ensure_link(store: &Path, at: &Path, rel: &Path) -> Result<Linked> {
    if link::is_linked_to(store, at) {
        return Ok(Linked::Yes);
    }
    if let Some(target) = link::read_target(at) {
        if !is_store_target(&target, rel) {
            tracing::debug!(at = %at.display(), "leaving a foreign link alone");
            return Ok(Linked::No(NotSharedReason::ForeignLink));
        }
        link::unlink_dir(at)?;
    } else if at.symlink_metadata().is_ok() {
        if !link::is_empty_dir(at) {
            let collisions = store_collisions(store, at)?;
            if !collisions.is_empty() {
                tracing::warn!(
                    at = %at.display(),
                    collisions = collisions.join(", "),
                    "not adopting a folder whose names the store already has"
                );
                return Ok(Linked::No(NotSharedReason::Collides));
            }
            adopt(store, at, rel, OnCollision::Refuse)?;
            tracing::info!(at = %at.display(), "adopted a folder into the shared store");
            return Ok(Linked::Yes);
        }
        fs::remove_dir(at)?;
    }
    make_link(store, at)?;
    Ok(Linked::Yes)
}

/// The adopt pass for one folder target: the instance's entries move into the
/// store and the emptied folder becomes a link. Collisions are settled before
/// any move, so a refused target has moved nothing. Returns the names the
/// store's copy won.
pub fn adopt(
    store: &Path,
    at: &Path,
    rel: &Path,
    on_collision: OnCollision,
) -> Result<Vec<String>> {
    if link::is_linked_to(store, at) {
        return Ok(Vec::new());
    }
    if let Some(target) = link::read_target(at) {
        if !is_store_target(&target, rel) {
            bail!(
                "{} is a link to {} — unlink it first",
                at.display(),
                target.display()
            );
        }
        link::unlink_dir(at)?;
        make_link(store, at)?;
        return Ok(Vec::new());
    }
    if !at.exists() || link::is_empty_dir(at) {
        if at.exists() {
            fs::remove_dir(at)?;
        }
        make_link(store, at)?;
        return Ok(Vec::new());
    }

    let collisions = store_collisions(store, at)?;
    if !collisions.is_empty() && on_collision == OnCollision::Refuse {
        bail!(
            "the store already has: {} (in {} — rename these, then retry)",
            collisions.join(", "),
            store.display()
        );
    }

    fs::create_dir_all(store).with_context(|| format!("cannot create {}", store.display()))?;
    for path in folder_entries(at)? {
        let name = path.file_name().context("entry without a name")?;
        let into = store.join(name);
        if into.symlink_metadata().is_ok() {
            discard(&path)?;
        } else {
            move_entry(&path, &into)?;
        }
    }
    fs::remove_dir(at).with_context(|| format!("cannot remove the emptied {}", at.display()))?;
    make_link(store, at)?;
    Ok(collisions)
}

/// Replace a link into the store with a real directory holding a copy of what
/// the store has — what leaving sharing does. Returns the bytes copied, or
/// `None` when the folder was not shared at all.
pub fn materialize(store: &Path, at: &Path, rel: &Path) -> Result<Option<u64>> {
    if !links_into_a_store(at, rel) {
        return Ok(None);
    }
    link::unlink_dir(at)?;
    if store.is_dir() {
        copy_tree(store, at)?;
    } else {
        fs::create_dir_all(at).with_context(|| format!("cannot create {}", at.display()))?;
    }
    Ok(Some(crate::usage::dir_size(at)))
}

/// Delete an entry the store's copy has replaced. A link is unlinked rather
/// than followed — the target is somebody else's data.
fn discard(path: &Path) -> Result<()> {
    if link::read_target(path).is_some() {
        return link::unlink_dir(path);
    }
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("cannot discard {}", path.display()))
}

/// One folder target's link state for a `data/` directory.
pub fn state(store: &Path, at: &Path, rel: &Path) -> LinkState {
    if link::is_linked_to(store, at) {
        return LinkState::Linked;
    }
    if let Some(target) = link::read_target(at) {
        return if is_store_target(&target, rel) {
            LinkState::Pending
        } else {
            LinkState::CannotLink
        };
    }
    if at.symlink_metadata().is_err() || link::is_empty_dir(at) {
        return LinkState::Pending;
    }
    // A folder with contents is adopted at the next launch unless the store
    // already holds one of its names.
    match store_collisions(store, at) {
        Ok(collisions) if collisions.is_empty() => LinkState::Pending,
        _ => LinkState::CannotLink,
    }
}

/// Whether the instance is already sharing this folder — a link into any hestia
/// store, including a stale one. A pack-owned target the user adopted by hand
/// reads as opted in and keeps reconciling.
pub fn links_into_a_store(at: &Path, rel: &Path) -> bool {
    link::read_target(at).is_some_and(|target| is_store_target(&target, rel))
}

/// Whether a link target points into *a* hestia store (this data home's, a
/// stale one after a data-home move, or a profile's captured store):
/// `…/shared/<rel>` or `…/profiles/<name>/<rel>`. Only such links are ever
/// touched; a user's own unrelated symlink is left alone.
fn is_store_target(target: &Path, rel: &Path) -> bool {
    if target.ends_with(Path::new("shared").join(rel)) {
        return true;
    }
    if !target.ends_with(rel) {
        return false;
    }
    let mut above = target;
    for _ in 0..rel.components().count() {
        match above.parent() {
            Some(parent) => above = parent,
            None => return false,
        }
    }
    above.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new("profiles"))
}

/// The names in `at` the store already holds — what an adopt would overwrite,
/// and the one thing that keeps a folder out of the store.
fn store_collisions(store: &Path, at: &Path) -> Result<Vec<String>> {
    Ok(folder_entries(at)?
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?;
            store
                .join(name)
                .symlink_metadata()
                .is_ok()
                .then(|| name.to_string_lossy().into_owned())
        })
        .collect())
}

fn folder_entries(at: &Path) -> Result<Vec<PathBuf>> {
    Ok(fs::read_dir(at)
        .with_context(|| format!("cannot read {}", at.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect())
}

fn make_link(store: &Path, at: &Path) -> Result<()> {
    fs::create_dir_all(store).with_context(|| format!("cannot create {}", store.display()))?;
    if let Some(parent) = at.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    link::link_dir(store, at)
}

/// Move one directory entry, falling back to copy-and-delete when a rename
/// crosses devices (the data home on another filesystem).
fn move_entry(from: &Path, to: &Path) -> Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_tree(from, to)
        .with_context(|| format!("cannot move {} to {}", from.display(), to.display()))?;
    if from.is_dir() && link::read_target(from).is_none() {
        fs::remove_dir_all(from)?;
    } else {
        fs::remove_file(from)?;
    }
    Ok(())
}

pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.is_dir() && link::read_target(from).is_none() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)?.flatten() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else if meta.is_file() {
        copy_file(from, to)?;
    } else {
        bail!(
            "cannot copy {} (not a regular file or directory)",
            from.display()
        );
    }
    Ok(())
}
