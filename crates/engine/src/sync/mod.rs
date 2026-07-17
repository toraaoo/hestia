//! Shared settings/configs, propagated across instances through a persistent
//! `<data_home>/shared/` store. Two target classes, following Pandora's split:
//!
//! - **Files are copied** (`options.txt` key-merged, others newest-wins): each
//!   instance keeps its own physical copy under `data/`, reconciled with the
//!   store at every launch. File symlinks would need elevation on Windows.
//! - **Folders are linked** (a symlink on POSIX, a junction on Windows) into
//!   the store, so folder content — worlds above all — is stored once and
//!   shared live between instances. A folder only becomes a link when it is
//!   missing, empty, or already linked into a hestia store (the
//!   empty-or-linked guard): a non-empty real directory is never merged or
//!   overwritten — it is reported `cannot_link` until `adopt` moves its
//!   entries into the store.
//!
//! Sync is **instance-only**: a client-side quality-of-life feature. A server's
//! configuration is per-server infrastructure (`server.config.*`,
//! `server.properties`) and is never shared — concurrent live servers must not
//! share writable config.
//!
//! The launcher-managed content directories (`mods/`, `resourcepacks/`,
//! `shaderpacks/`) are off-limits as targets: the content system owns them and
//! per-instance selection is impossible over a shared directory. `saves/` is a
//! valid — and default — *linked* target, but stays invalid as a copied one.

pub(crate) mod link;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{bail, Context, Result};
use proto::sync::{LinkState, SyncTargets, TargetLinkState};

const TARGETS_FILE: &str = "targets.json";
const OPTIONS_TXT: &str = "options.txt";

/// First-path-component names no target may ever claim: the launcher-managed
/// content dirs (owned by the content system) and the backups dir.
const RESERVED_ROOTS: &[&str] = &["mods", "resourcepacks", "shaderpacks", "backups"];

/// `options.txt` keys kept entry-local — pack selection must not leak between
/// instances through the shared store (mirrors Pandora's `options.txt` handling).
const LOCAL_OPTION_KEYS: &[&str] = &["resourcePacks", "incompatibleResourcePacks"];

pub struct Sync {
    dir: Mutex<PathBuf>,
}

impl Sync {
    pub fn new(dir: PathBuf) -> Self {
        Sync {
            dir: Mutex::new(dir),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    /// The shared store root (`<data_home>/shared`).
    pub fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    /// The current target set — the persisted file, or the built-in defaults
    /// when none has been written yet.
    pub fn targets(&self) -> SyncTargets {
        let path = self.dir().join(TARGETS_FILE);
        fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(default_targets)
    }

    /// Validate and persist a new target set. Each path must be relative,
    /// free of `..` escapes, and outside the launcher-managed directories;
    /// `saves` is additionally rejected as a *file* (copied) target.
    pub fn set_targets(&self, targets: SyncTargets) -> Result<SyncTargets> {
        for path in &targets.files {
            validate_target(path)?;
            if first_component(path).as_deref() == Some("saves") {
                bail!("'{path}' cannot be a copied target (share `saves` as a folder instead)");
            }
        }
        for path in &targets.folders {
            validate_target(path)?;
        }
        let dir = self.dir();
        fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
        let text = serde_json::to_string_pretty(&targets).expect("SyncTargets serializes");
        fs::write(dir.join(TARGETS_FILE), format!("{text}\n"))?;
        Ok(targets)
    }

    /// Reconcile an instance's `data/` with the shared store: copy-reconcile
    /// the file targets newest-wins, then ensure each folder target is a link
    /// into the store (the apply pass). Best-effort per target: a single
    /// failing target is logged and skipped rather than failing the launch.
    pub fn apply(&self, data_dir: &Path) -> Result<()> {
        let targets = self.targets();
        let shared = self.dir();
        fs::create_dir_all(&shared).ok();

        for rel in &targets.files {
            let Some(rel) = safe_rel(rel) else { continue };
            let result = if rel.as_os_str() == OPTIONS_TXT {
                merge_options(&shared.join(&rel), &data_dir.join(&rel))
            } else {
                sync_newer(&shared.join(&rel), &data_dir.join(&rel))
            };
            if let Err(e) = result {
                tracing::warn!(target = %rel.display(), error = %e, "config sync skipped a file");
            }
        }

        for rel in &targets.folders {
            let Some(rel) = safe_rel(rel) else { continue };
            if let Err(e) = ensure_link(&shared.join(&rel), &data_dir.join(&rel), &rel) {
                tracing::warn!(
                    target = %rel.display(),
                    error = format!("{e:#}"),
                    "cannot link a sync folder"
                );
            }
        }
        Ok(())
    }

    /// Each folder target's link state for one instance's `data/`.
    pub fn status(&self, data_dir: &Path) -> Vec<TargetLinkState> {
        let targets = self.targets();
        let shared = self.dir();
        targets
            .folders
            .iter()
            .filter_map(|raw| {
                let rel = safe_rel(raw)?;
                let state = link_state(&shared.join(&rel), &data_dir.join(&rel), &rel);
                Some(TargetLinkState {
                    target: raw.clone(),
                    state,
                })
            })
            .collect()
    }

    /// Move the entries of an instance's real folder targets into the store
    /// and link the emptied folders. All-or-nothing per target: any name that
    /// already exists in the store refuses that whole target, naming the
    /// collisions — nothing is ever merged or overwritten. Returns each
    /// target that is linked after the call.
    pub fn adopt(&self, data_dir: &Path, requested: &[String]) -> Result<Vec<String>> {
        let targets = self.targets();
        let all: Vec<String> = if requested.is_empty() {
            targets.folders.iter().cloned().collect()
        } else {
            for name in requested {
                if !targets.folders.contains(name) {
                    bail!("'{name}' is not a folder sync target");
                }
            }
            requested.to_vec()
        };

        let shared = self.dir();
        let mut adopted = Vec::new();
        for raw in all {
            let Some(rel) = safe_rel(&raw) else { continue };
            let store = shared.join(&rel);
            let at = data_dir.join(&rel);
            adopt_folder(&store, &at, &rel).with_context(|| format!("cannot adopt '{raw}'"))?;
            adopted.push(raw);
        }
        Ok(adopted)
    }
}

/// The built-in targets. Copied files: `options.txt` (keybinds/video,
/// key-merged) and `servers.dat` (the multiplayer list). Linked folders:
/// `saves` (the shared worlds), mod `config/`, and `screenshots`.
fn default_targets() -> SyncTargets {
    SyncTargets {
        files: [OPTIONS_TXT.to_string(), "servers.dat".to_string()]
            .into_iter()
            .collect(),
        folders: [
            "saves".to_string(),
            "config".to_string(),
            "screenshots".to_string(),
        ]
        .into_iter()
        .collect(),
    }
}

/// The apply pass for one folder target: nothing to do when already linked;
/// a stale hestia-store link (the data home moved) is relinked; a missing or
/// empty directory becomes a link; a non-empty real directory — or a foreign
/// link the user made — is left untouched (Pandora's empty-or-linked guard).
fn ensure_link(store: &Path, at: &Path, rel: &Path) -> Result<()> {
    if link::is_linked_to(store, at) {
        return Ok(());
    }
    if let Some(target) = link::read_target(at) {
        if !is_store_target(&target, rel) {
            tracing::debug!(at = %at.display(), "leaving a foreign link alone");
            return Ok(());
        }
        link::unlink_dir(at)?;
    } else if at.symlink_metadata().is_ok() {
        if !link::is_empty_dir(at) {
            tracing::warn!(
                at = %at.display(),
                "not linking a non-empty directory (run `sync adopt` to move it into the store)"
            );
            return Ok(());
        }
        fs::remove_dir(at)?;
    }
    make_link(store, at)
}

/// The adopt pass for one folder target. Collision checks run before any move,
/// so a refused target has moved nothing.
fn adopt_folder(store: &Path, at: &Path, rel: &Path) -> Result<()> {
    if link::is_linked_to(store, at) {
        return Ok(());
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
        return make_link(store, at);
    }
    if !at.exists() || link::is_empty_dir(at) {
        if at.exists() {
            fs::remove_dir(at)?;
        }
        return make_link(store, at);
    }

    let entries: Vec<PathBuf> = fs::read_dir(at)
        .with_context(|| format!("cannot read {}", at.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    let collisions: Vec<String> = entries
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?;
            store
                .join(name)
                .symlink_metadata()
                .is_ok()
                .then(|| name.to_string_lossy().into_owned())
        })
        .collect();
    if !collisions.is_empty() {
        bail!(
            "the store already has: {} (in {} — rename these, then retry)",
            collisions.join(", "),
            store.display()
        );
    }

    fs::create_dir_all(store).with_context(|| format!("cannot create {}", store.display()))?;
    for path in entries {
        let name = path.file_name().context("entry without a name")?;
        move_entry(&path, &store.join(name))?;
    }
    fs::remove_dir(at).with_context(|| format!("cannot remove the emptied {}", at.display()))?;
    make_link(store, at)
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

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.is_dir() && link::read_target(from).is_none() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)?.flatten() {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else if meta.is_file() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
    } else {
        bail!(
            "cannot copy {} (not a regular file or directory)",
            from.display()
        );
    }
    Ok(())
}

fn link_state(store: &Path, at: &Path, rel: &Path) -> LinkState {
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
        LinkState::Pending
    } else {
        LinkState::CannotLink
    }
}

/// Whether a link target points into *a* hestia shared store (this data home's
/// or a stale one after a data-home move): `…/shared/<rel>`. Only such links
/// are ever touched; a user's own unrelated symlink is left alone.
fn is_store_target(target: &Path, rel: &Path) -> bool {
    target.ends_with(Path::new("shared").join(rel))
}

/// Reject an absolute path, a `..` escape, an empty path, or one rooted at a
/// launcher-managed directory.
fn validate_target(path: &str) -> Result<()> {
    let first =
        first_component(path).with_context(|| format!("'{path}' is not a safe relative path"))?;
    if RESERVED_ROOTS.contains(&first.as_str()) {
        bail!("'{path}' is a launcher-managed directory and cannot be a sync target");
    }
    Ok(())
}

fn first_component(path: &str) -> Option<String> {
    safe_rel(path)?.components().find_map(|c| match c {
        Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
        _ => None,
    })
}

/// Normalise a target string to a relative path, rejecting absolute paths and any
/// component that escapes the root (`..`, a root/prefix).
fn safe_rel(path: &str) -> Option<PathBuf> {
    let candidate = Path::new(path);
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if normalized.as_os_str().is_empty() {
        return None;
    }
    Some(normalized)
}

/// Copy whichever of `a`/`b` is newer onto the other. If only one exists, copy it
/// across. Missing on both sides is a no-op.
fn sync_newer(a: &Path, b: &Path) -> Result<()> {
    match (mtime(a), mtime(b)) {
        (Some(ta), Some(tb)) if ta > tb => copy_file(a, b),
        (Some(_), Some(_)) => copy_file(b, a),
        (Some(_), None) => copy_file(a, b),
        (None, Some(_)) => copy_file(b, a),
        (None, None) => Ok(()),
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::copy(from, to)
        .with_context(|| format!("cannot copy {} to {}", from.display(), to.display()))?;
    Ok(())
}

/// Merge `options.txt` key-by-key. The newer file's values win on conflict; the
/// union of keys is written to both sides. Pack-selection keys stay entry-local:
/// the entry keeps its own values, and they never propagate into the store.
fn merge_options(shared: &Path, data: &Path) -> Result<()> {
    let shared_vals = read_options(shared);
    let data_vals = read_options(data);
    if shared_vals.is_empty() && data_vals.is_empty() {
        return Ok(());
    }

    let data_newer = match (mtime(data), mtime(shared)) {
        (Some(td), Some(ts)) => td >= ts,
        (Some(_), None) => true,
        _ => false,
    };
    let (mut merged, overlay) = if data_newer {
        (shared_vals.clone(), data_vals.clone())
    } else {
        (data_vals.clone(), shared_vals.clone())
    };
    for (key, value) in overlay {
        merged.insert(key, value);
    }

    let mut data_out = merged.clone();
    for key in LOCAL_OPTION_KEYS {
        match data_vals.get(*key) {
            Some(value) => data_out.insert(key.to_string(), value.clone()),
            None => data_out.remove(*key),
        };
    }
    let mut shared_out = merged;
    for key in LOCAL_OPTION_KEYS {
        shared_out.remove(*key);
    }

    write_options(data, &data_out)?;
    write_options(shared, &shared_out)?;
    Ok(())
}

fn read_options(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|line| line.trim().split_once(':'))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn write_options(path: &Path, values: &BTreeMap<String, String>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    let mut text = String::new();
    for (key, value) in values {
        text.push_str(key);
        text.push(':');
        text.push_str(value);
        text.push('\n');
    }
    fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("hestia-sync-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn validate_rejects_managed_dirs_and_escapes() {
        assert!(validate_target("mods/sodium.jar").is_err());
        assert!(validate_target("resourcepacks/x").is_err());
        assert!(validate_target("../secret").is_err());
        assert!(validate_target("/etc/passwd").is_err());
        assert!(validate_target("").is_err());
        assert!(validate_target("options.txt").is_ok());
        assert!(validate_target("config/mod.toml").is_ok());
        assert!(validate_target("saves").is_ok());
    }

    #[test]
    fn saves_is_a_folder_target_only() {
        let base = temp_dir("savesclass");
        let sync = Sync::new(base.join("shared"));

        let mut targets = SyncTargets::default();
        targets.folders.insert("saves".to_string());
        assert!(sync.set_targets(targets).is_ok());

        let mut targets = SyncTargets::default();
        targets.files.insert("saves".to_string());
        assert!(sync.set_targets(targets).is_err());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn seeds_a_new_instance_from_the_store() {
        let base = temp_dir("seed");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("options.txt"), "guiScale:3\n").unwrap();

        Sync::new(shared).apply(&data).unwrap();

        let seeded = fs::read_to_string(data.join("options.txt")).unwrap();
        assert!(seeded.contains("guiScale:3"));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn pack_selection_stays_entry_local() {
        let base = temp_dir("packs");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(
            data.join("options.txt"),
            "guiScale:2\nresourcePacks:[\"cozy\"]\n",
        )
        .unwrap();

        Sync::new(shared.clone()).apply(&data).unwrap();

        let stored = fs::read_to_string(shared.join("options.txt")).unwrap();
        assert!(stored.contains("guiScale:2"));
        assert!(
            !stored.contains("resourcePacks"),
            "pack selection must not propagate to the shared store"
        );
        // The entry keeps its own pack selection.
        let local = fs::read_to_string(data.join("options.txt")).unwrap();
        assert!(local.contains("resourcePacks"));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn apply_links_missing_and_empty_folders() {
        let base = temp_dir("linkfresh");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(data.join("config")).unwrap();

        Sync::new(shared.clone()).apply(&data).unwrap();

        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));
        assert!(link::is_linked_to(
            &shared.join("config"),
            &data.join("config")
        ));

        // A world created through one instance's link is visible in another's.
        fs::create_dir_all(data.join("saves").join("world")).unwrap();
        let data2 = base.join("data2");
        fs::create_dir_all(&data2).unwrap();
        Sync::new(shared.clone()).apply(&data2).unwrap();
        assert!(data2.join("saves").join("world").is_dir());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn apply_never_touches_a_non_empty_folder() {
        let base = temp_dir("guard");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(data.join("saves").join("old-world")).unwrap();
        fs::write(data.join("saves").join("old-world").join("level.dat"), "x").unwrap();

        let sync = Sync::new(shared.clone());
        sync.apply(&data).unwrap();

        assert!(link::read_target(&data.join("saves")).is_none());
        assert!(data
            .join("saves")
            .join("old-world")
            .join("level.dat")
            .exists());
        let states = sync.status(&data);
        let saves = states.iter().find(|t| t.target == "saves").unwrap();
        assert_eq!(saves.state, LinkState::CannotLink);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn apply_relinks_a_stale_store_link() {
        let base = temp_dir("stale");
        let old_shared = base.join("old-home").join("shared");
        fs::create_dir_all(old_shared.join("saves")).unwrap();
        let data = base.join("data");
        fs::create_dir_all(&data).unwrap();
        link::link_dir(&old_shared.join("saves"), &data.join("saves")).unwrap();

        let shared = base.join("new-home").join("shared");
        Sync::new(shared.clone()).apply(&data).unwrap();

        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn apply_leaves_a_foreign_link_alone() {
        let base = temp_dir("foreign");
        let shared = base.join("shared");
        let elsewhere = base.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let data = base.join("data");
        fs::create_dir_all(&data).unwrap();
        link::link_dir(&elsewhere, &data.join("saves")).unwrap();

        Sync::new(shared.clone()).apply(&data).unwrap();

        assert_eq!(link::read_target(&data.join("saves")), Some(elsewhere));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn adopt_moves_entries_and_links() {
        let base = temp_dir("adopt");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(data.join("saves").join("world-a")).unwrap();
        fs::write(data.join("saves").join("world-a").join("level.dat"), "a").unwrap();
        fs::create_dir_all(data.join("saves").join("world-b")).unwrap();

        let sync = Sync::new(shared.clone());
        let adopted = sync.adopt(&data, &["saves".to_string()]).unwrap();

        assert_eq!(adopted, vec!["saves".to_string()]);
        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));
        assert!(shared
            .join("saves")
            .join("world-a")
            .join("level.dat")
            .exists());
        assert!(shared.join("saves").join("world-b").is_dir());
        // Both worlds open through the link.
        assert!(data
            .join("saves")
            .join("world-a")
            .join("level.dat")
            .exists());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn adopt_refuses_the_whole_target_on_collision() {
        let base = temp_dir("collide");
        let shared = base.join("shared");
        fs::create_dir_all(shared.join("saves").join("world")).unwrap();
        fs::write(
            shared.join("saves").join("world").join("level.dat"),
            "store",
        )
        .unwrap();
        let data = base.join("data");
        fs::create_dir_all(data.join("saves").join("world")).unwrap();
        fs::write(data.join("saves").join("world").join("level.dat"), "mine").unwrap();
        fs::create_dir_all(data.join("saves").join("other")).unwrap();

        let sync = Sync::new(shared.clone());
        let err = sync.adopt(&data, &["saves".to_string()]).unwrap_err();
        assert!(format!("{err:#}").contains("world"));

        // Nothing moved — not even the non-colliding entry.
        assert!(data.join("saves").join("other").is_dir());
        assert!(link::read_target(&data.join("saves")).is_none());
        assert_eq!(
            fs::read_to_string(shared.join("saves").join("world").join("level.dat")).unwrap(),
            "store"
        );
        fs::remove_dir_all(&base).ok();
    }
}
