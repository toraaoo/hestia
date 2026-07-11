//! Shared settings/configs. A small set of game-relative settings files/folders
//! is propagated across entries through a persistent `<data_home>/shared/` store.
//!
//! It is deliberately **copy-based, not symlinked**: each entry keeps its own
//! physical copy under `data/`, and `apply` reconciles that copy with the shared
//! store newest-wins at every start/launch. Nothing is live-shared between two
//! running entries, so concurrent writers can't corrupt each other and backups
//! (which archive `data/`) stay intact. Convergence is emergent — the first
//! entry to launch seeds the store, later entries inherit it.
//!
//! Targets and the store are **kept separate per entry kind** (`shared/servers/`,
//! `shared/instances/`): unlike Pandora (client-only), Hestia manages both, and a
//! server syncs different files than a client (a server has no `options.txt`) —
//! and a server's mod `config/` must never bleed into a client's.
//!
//! Scope is settings/config only. The launcher-managed content directories
//! (`mods/`, `resourcepacks/`, `shaderpacks/`) are off-limits (the content system
//! already shares those), and so is `saves/` (worlds belong to the backup
//! system). Those are rejected as targets at the edge.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{bail, Context, Result};
use proto::sync::{SyncKind, SyncTargets};

const TARGETS_FILE: &str = "targets.json";
const OPTIONS_TXT: &str = "options.txt";

/// First-path-component names a target may never claim: the launcher-managed
/// content dirs (shared by the content system) and the world/backup dirs.
const RESERVED_ROOTS: &[&str] = &["mods", "resourcepacks", "shaderpacks", "saves", "backups"];

/// `options.txt` keys kept entry-local — pack selection must not leak between
/// entries through the shared store (mirrors Pandora's `options.txt` handling).
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

    /// The per-kind store subdirectory (`shared/servers` / `shared/instances`).
    fn kind_dir(&self, kind: SyncKind) -> PathBuf {
        self.dir().join(match kind {
            SyncKind::Server => "servers",
            SyncKind::Instance => "instances",
        })
    }

    /// A kind's current target set — the persisted file, or the built-in defaults
    /// when none has been written yet.
    pub fn targets(&self, kind: SyncKind) -> SyncTargets {
        let path = self.kind_dir(kind).join(TARGETS_FILE);
        fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| default_targets(kind))
    }

    /// Validate and persist a kind's new target set. Each path must be relative,
    /// free of `..` escapes, and outside the launcher-managed directories.
    pub fn set_targets(&self, kind: SyncKind, targets: SyncTargets) -> Result<SyncTargets> {
        for path in targets.files.iter().chain(targets.folders.iter()) {
            validate_target(path)?;
        }
        let dir = self.kind_dir(kind);
        fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
        let text = serde_json::to_string_pretty(&targets).expect("SyncTargets serializes");
        fs::write(dir.join(TARGETS_FILE), format!("{text}\n"))?;
        Ok(targets)
    }

    /// Reconcile an entry's `data/` with its kind's shared store, newest-wins per
    /// target. Best-effort per target: a single failing file is logged and skipped
    /// rather than failing the launch. A no-op when the entry has opted out.
    pub fn apply(&self, data_dir: &Path, kind: SyncKind, opted_in: bool) -> Result<()> {
        if !opted_in {
            return Ok(());
        }
        let targets = self.targets(kind);
        let shared = self.kind_dir(kind);
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
            if let Err(e) = sync_folder(&shared.join(&rel), &data_dir.join(&rel)) {
                tracing::warn!(target = %rel.display(), error = %e, "config sync skipped a folder");
            }
        }
        Ok(())
    }
}

/// The built-in targets for a kind. Both share mod `config/`; only a client has
/// `options.txt` (keybinds/video) and `servers.dat` (the multiplayer list).
fn default_targets(kind: SyncKind) -> SyncTargets {
    let folders = ["config".to_string()].into_iter().collect();
    match kind {
        SyncKind::Server => SyncTargets {
            files: Default::default(),
            folders,
        },
        SyncKind::Instance => SyncTargets {
            files: [OPTIONS_TXT.to_string(), "servers.dat".to_string()]
                .into_iter()
                .collect(),
            folders,
        },
    }
}

/// Reject an absolute path, a `..` escape, an empty path, or one rooted at a
/// launcher-managed directory.
fn validate_target(path: &str) -> Result<()> {
    let rel = safe_rel(path).with_context(|| format!("'{path}' is not a safe relative path"))?;
    let first = rel
        .components()
        .find_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .with_context(|| format!("'{path}' is empty"))?;
    if RESERVED_ROOTS.contains(&first.as_str()) {
        bail!("'{path}' is a launcher-managed directory and cannot be a sync target");
    }
    Ok(())
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

/// Sync every file that exists under either folder, keyed by relative path.
fn sync_folder(shared: &Path, data: &Path) -> Result<()> {
    let mut rels = std::collections::BTreeSet::new();
    collect_files(shared, shared, &mut rels)?;
    collect_files(data, data, &mut rels)?;
    for rel in rels {
        sync_newer(&shared.join(&rel), &data.join(&rel))?;
    }
    Ok(())
}

fn collect_files(
    root: &Path,
    dir: &Path,
    out: &mut std::collections::BTreeSet<PathBuf>,
) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            collect_files(root, &path, out)?;
        } else if file_type.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                out.insert(rel.to_path_buf());
            }
        }
    }
    Ok(())
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
        assert!(validate_target("saves/world").is_err());
        assert!(validate_target("../secret").is_err());
        assert!(validate_target("/etc/passwd").is_err());
        assert!(validate_target("").is_err());
        assert!(validate_target("options.txt").is_ok());
        assert!(validate_target("config/mod.toml").is_ok());
    }

    #[test]
    fn seeds_a_new_entry_from_the_store() {
        let base = temp_dir("seed");
        let shared = base.join("shared");
        let data = base.join("data");
        let store = shared.join("instances");
        fs::create_dir_all(&store).unwrap();
        fs::write(store.join("options.txt"), "guiScale:3\n").unwrap();

        Sync::new(shared)
            .apply(&data, SyncKind::Instance, true)
            .unwrap();

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

        Sync::new(shared.clone())
            .apply(&data, SyncKind::Instance, true)
            .unwrap();

        let stored = fs::read_to_string(shared.join("instances").join("options.txt")).unwrap();
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
    fn folder_targets_sync_per_file() {
        let base = temp_dir("folder");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(data.join("config")).unwrap();
        fs::write(data.join("config").join("mod.toml"), "x=1").unwrap();

        Sync::new(shared.clone())
            .apply(&data, SyncKind::Server, true)
            .unwrap();

        assert_eq!(
            fs::read_to_string(shared.join("servers").join("config").join("mod.toml")).unwrap(),
            "x=1"
        );
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn server_and_instance_stores_do_not_mix() {
        let base = temp_dir("kindsplit");
        let shared = base.join("shared");
        let server_data = base.join("srv");
        let instance_data = base.join("inst");
        let sync = Sync::new(shared.clone());

        fs::create_dir_all(server_data.join("config")).unwrap();
        fs::write(server_data.join("config").join("mod.toml"), "side=server").unwrap();
        fs::create_dir_all(instance_data.join("config")).unwrap();
        fs::write(instance_data.join("config").join("mod.toml"), "side=client").unwrap();

        sync.apply(&server_data, SyncKind::Server, true).unwrap();
        sync.apply(&instance_data, SyncKind::Instance, true)
            .unwrap();

        // The client's config is untouched by the server sync (separate stores).
        assert_eq!(
            fs::read_to_string(instance_data.join("config").join("mod.toml")).unwrap(),
            "side=client"
        );
        assert_eq!(
            fs::read_to_string(shared.join("servers").join("config").join("mod.toml")).unwrap(),
            "side=server"
        );
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn opt_out_is_a_no_op() {
        let base = temp_dir("optout");
        let shared = base.join("shared");
        let data = base.join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("options.txt"), "guiScale:1\n").unwrap();

        Sync::new(shared.clone())
            .apply(&data, SyncKind::Instance, false)
            .unwrap();

        assert!(!shared.join("instances").join("options.txt").exists());
        fs::remove_dir_all(&base).ok();
    }
}
