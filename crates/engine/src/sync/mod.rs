//! Shared settings/configs, propagated across instances through a persistent
//! `<data_home>/shared/` store. Two target classes, following Pandora's split:
//!
//! - **Files are copied** ([`files`]): each instance keeps its own physical copy
//!   under `data/`, reconciled against a per-instance baseline. File symlinks
//!   would need elevation on Windows.
//! - **Folders are linked** ([`folders`]): a symlink on POSIX, a junction on
//!   Windows, so folder content — worlds above all — is stored once and shared
//!   live between instances.
//!
//! A [`Pass`] is one reconcile: which instance, where its game directory is, and
//! which store its settings-class targets belong to. A launch runs one and
//! remembers it; the same pass runs again when that session exits, so what the
//! player changed in game reaches the store then rather than at their next
//! launch.
//!
//! Sharing is switchable launcher-wide (`sync.enabled`) and per instance
//! ([`Sync::attach`] / [`Sync::detach`]): off, no pass runs at all. Links
//! already made are left alone — hestia never breaks one behind the user's
//! back.
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

mod files;
mod folders;
pub(crate) mod link;

use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::sync::{SyncTargets, TargetLinkState};
use proto::warning::WarningInfo;

use folders::Linked;

const TARGETS_FILE: &str = "targets.json";
const OPTIONS_TXT: &str = "options.txt";

/// Where a store keeps, per instance, the content each copied target last
/// agreed on.
const BASELINES: &str = ".baselines";

/// First-path-component names no target may ever claim: the launcher-managed
/// content dirs (owned by the content system), the backups dir, and the store's
/// own bookkeeping.
const RESERVED_ROOTS: &[&str] = &["mods", "resourcepacks", "shaderpacks", "backups", BASELINES];

/// The settings-class targets a captured profile scopes to its own store.
/// Worlds and screenshots stay on the global store: capture exists to fork
/// *settings*, not game data.
const CAPTURE_FILES: &[&str] = &[OPTIONS_TXT];
const CAPTURE_FOLDERS: &[&str] = &["config"];

/// Where an instance's settings-class targets reconcile for one pass.
#[derive(Clone, Default, PartialEq, Eq)]
pub enum Scope {
    /// The global shared store — the ordinary instance.
    #[default]
    Shared,
    /// A captured profile's own store.
    Profile(PathBuf),
    /// The instance owns them: a modpack ships its config tree, so hestia does
    /// not link or adopt one behind the user's back. An `adopt` the user asks
    /// for still opts in, and the link it leaves is honoured from then on.
    Local,
}

impl Scope {
    /// Whether this target is one the instance keeps to itself unless the user
    /// has already opted in by adopting it.
    fn owns_locally(&self, target: &str) -> bool {
        matches!(self, Scope::Local) && CAPTURE_FOLDERS.contains(&target)
    }
}

/// One reconcile of one instance. The id keys its baselines and the name is
/// what a warning calls it, so both travel together — a rename must not read as
/// a different instance to the store.
#[derive(Clone)]
pub struct Pass {
    pub id: String,
    pub name: String,
    pub data_dir: PathBuf,
    pub scope: Scope,
}

pub struct Sync {
    dir: Mutex<PathBuf>,
    /// What each live session reconciles against, so its exit pass uses the
    /// scope it launched under rather than whichever is active by then. A daemon
    /// restart drops these; the next launch reconciles as it always would.
    sessions: Mutex<HashMap<String, Pass>>,
}

impl Sync {
    pub fn new(dir: PathBuf) -> Self {
        Sync {
            dir: Mutex::new(dir),
            sessions: Mutex::new(HashMap::new()),
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
                bail!(proto::error::ErrorInfo::SyncTargetInvalid {
                    path: path.to_string(),
                    reason: proto::error::SyncReason::CopiedTarget
                });
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

    /// Reconcile an instance's `data/` with the store: settle the file targets
    /// against their baselines, then ensure each folder target is a link.
    ///
    /// Best-effort per target — refusing to launch over a leftover folder would
    /// be worse than launching unshared. Every skip is **returned** as a
    /// warning: the user configured that target expecting it to be shared, and
    /// would otherwise play against the wrong data with no sign of it.
    pub fn apply(&self, pass: &Pass) -> Vec<WarningInfo> {
        let targets = self.targets();
        let shared = self.dir();
        fs::create_dir_all(&shared).ok();
        let mut warnings = Vec::new();

        for raw in &targets.files {
            let Some(rel) = safe_rel(raw) else { continue };
            let store = scope_root(&shared, &pass.scope, raw, CAPTURE_FILES);
            let baseline = baseline_path(&store, &pass.id, &rel);
            let store = store.join(&rel);
            let at = pass.data_dir.join(&rel);
            let result = if rel.as_os_str() == OPTIONS_TXT {
                files::merge_options(&baseline, &store, &at)
            } else {
                files::reconcile(&baseline, &store, &at)
            };
            if let Err(e) = result {
                tracing::warn!(target = %rel.display(), error = %e, "config sync skipped a file");
                warnings.push(WarningInfo::SyncTargetSkipped {
                    target: raw.clone(),
                    detail: format!("{e:#}"),
                });
            }
        }

        for raw in &targets.folders {
            let Some(rel) = safe_rel(raw) else { continue };
            let store = scope_root(&shared, &pass.scope, raw, CAPTURE_FOLDERS);
            let at = pass.data_dir.join(&rel);
            if pass.scope.owns_locally(raw) && !folders::links_into_a_store(&at, &rel) {
                tracing::debug!(target = %rel.display(), "leaving a pack-owned folder local");
                continue;
            }
            match folders::ensure_link(&store.join(&rel), &at, &rel) {
                Ok(Linked::Yes) => {}
                Ok(Linked::No(reason)) => warnings.push(WarningInfo::SyncTargetNotShared {
                    instance: pass.name.clone(),
                    target: raw.clone(),
                    reason,
                }),
                Err(e) => {
                    tracing::warn!(
                        target = %rel.display(),
                        error = format!("{e:#}"),
                        "cannot link a sync folder"
                    );
                    warnings.push(WarningInfo::SyncTargetSkipped {
                        target: raw.clone(),
                        detail: format!("{e:#}"),
                    });
                }
            }
        }
        warnings
    }

    /// Take an instance out of sharing: every folder it shares becomes its own
    /// copy of the store's content and its agreements are dropped. Nothing is
    /// deleted — it plays what it played before, and the two copies diverge
    /// from here.
    pub fn detach(&self, pass: &Pass) -> Result<Vec<WarningInfo>> {
        let targets = self.targets();
        let shared = self.dir();
        let mut warnings = Vec::new();
        for raw in &targets.folders {
            let Some(rel) = safe_rel(raw) else { continue };
            let store = scope_root(&shared, &pass.scope, raw, CAPTURE_FOLDERS).join(&rel);
            let at = pass.data_dir.join(&rel);
            if let Some(bytes) = folders::materialize(&store, &at, &rel)
                .with_context(|| format!("cannot copy '{raw}' out of the store"))?
            {
                tracing::info!(instance = %pass.name, target = %raw, bytes, "copied out of the store");
                warnings.push(WarningInfo::SyncTargetDuplicated {
                    target: raw.clone(),
                    bytes,
                });
            }
        }
        self.forget(&pass.id);
        Ok(warnings)
    }

    /// Bring an instance back into sharing. The store is the authority for
    /// anything the two both have, since the others are already playing it: a
    /// clashing folder entry keeps the store's copy and a clashing setting takes
    /// the store's value. What only this instance has is carried in.
    pub fn attach(&self, pass: &Pass) -> Result<Vec<WarningInfo>> {
        let targets = self.targets();
        let shared = self.dir();
        let mut warnings = Vec::new();
        for raw in &targets.files {
            let Some(rel) = safe_rel(raw) else { continue };
            let store = scope_root(&shared, &pass.scope, raw, CAPTURE_FILES);
            files::defer_to_store(
                &baseline_path(&store, &pass.id, &rel),
                &pass.data_dir.join(&rel),
            )
            .with_context(|| format!("cannot record the agreement for '{raw}'"))?;
        }
        for raw in &targets.folders {
            let Some(rel) = safe_rel(raw) else { continue };
            let store = scope_root(&shared, &pass.scope, raw, CAPTURE_FOLDERS).join(&rel);
            let at = pass.data_dir.join(&rel);
            let replaced = folders::adopt(&store, &at, &rel, folders::OnCollision::KeepStore)
                .with_context(|| format!("cannot share '{raw}'"))?;
            if !replaced.is_empty() {
                tracing::info!(
                    instance = %pass.name,
                    target = %raw,
                    replaced = replaced.join(", "),
                    "the store's copies won a clash"
                );
                warnings.push(WarningInfo::SyncEntriesReplaced {
                    target: raw.clone(),
                    entries: replaced,
                });
            }
        }
        warnings.extend(self.apply(pass));
        Ok(warnings)
    }

    /// Record what a starting session reconciled, keyed by its process id.
    pub fn remember(&self, session: &str, pass: Pass) {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.to_string(), pass);
    }

    /// Take back what a finished session reconciled, if it recorded anything.
    pub fn recall(&self, session: &str) -> Option<Pass> {
        self.sessions.lock().unwrap().remove(session)
    }

    /// Drop an instance's baselines from the global store. A profile's store
    /// lives under the instance and goes with it.
    pub fn forget(&self, id: &str) {
        let dir = self.dir().join(BASELINES).join(id);
        if dir.exists() {
            if let Err(e) = fs::remove_dir_all(&dir) {
                tracing::warn!(instance = id, error = %e, "cannot drop the sync baselines");
            }
        }
    }

    /// Seed a profile's captured store from the global one: the settings-class
    /// file and folder targets are copied as they currently stand. From then on
    /// launches under the profile reconcile against the captured store, and
    /// divergence is by design.
    pub fn capture(&self, profile_store: &Path) -> Result<()> {
        let targets = self.targets();
        let shared = self.dir();
        fs::create_dir_all(profile_store)
            .with_context(|| format!("cannot create {}", profile_store.display()))?;
        for raw in &targets.files {
            let Some(rel) = safe_rel(raw) else { continue };
            if !CAPTURE_FILES.contains(&raw.as_str()) {
                continue;
            }
            let source = shared.join(&rel);
            if source.is_file() {
                files::copy_file(&source, &profile_store.join(&rel))?;
            }
        }
        for raw in &targets.folders {
            let Some(rel) = safe_rel(raw) else { continue };
            if !CAPTURE_FOLDERS.contains(&raw.as_str()) {
                continue;
            }
            let source = shared.join(&rel);
            let dest = profile_store.join(&rel);
            if source.is_dir() && link::read_target(&source).is_none() {
                folders::copy_tree(&source, &dest)?;
            } else {
                fs::create_dir_all(&dest)
                    .with_context(|| format!("cannot create {}", dest.display()))?;
            }
        }
        Ok(())
    }

    /// Delete a profile's captured store; the profile inherits the global
    /// store again (the stale link in `data/` is relinked at the next apply).
    pub fn release(&self, profile_store: &Path) -> Result<()> {
        if profile_store.symlink_metadata().is_ok() {
            fs::remove_dir_all(profile_store)
                .with_context(|| format!("cannot remove {}", profile_store.display()))?;
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
                Some(TargetLinkState {
                    target: raw.clone(),
                    state: folders::state(&shared.join(&rel), &data_dir.join(&rel), &rel),
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
                    bail!(proto::error::ErrorInfo::SyncTargetInvalid {
                        path: name.to_string(),
                        reason: proto::error::SyncReason::NotFolderTarget
                    });
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
            folders::adopt(&store, &at, &rel, folders::OnCollision::Refuse)
                .with_context(|| format!("cannot adopt '{raw}'"))?;
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

/// The store root a target reconciles against: the profile's captured store
/// for settings-class targets when one is in scope, the global store otherwise.
fn scope_root(shared: &Path, scope: &Scope, raw: &str, scoped: &[&str]) -> PathBuf {
    match scope {
        Scope::Profile(store) if scoped.contains(&raw) => store.clone(),
        _ => shared.to_path_buf(),
    }
}

/// A baseline lives in the store it describes an agreement with, so a profile's
/// captured store carries its own and `release` takes them with it.
fn baseline_path(store: &Path, instance: &str, rel: &Path) -> PathBuf {
    store.join(BASELINES).join(instance).join(rel)
}

/// Reject an absolute path, a `..` escape, an empty path, or one rooted at a
/// launcher-managed directory.
fn validate_target(path: &str) -> Result<()> {
    let first =
        first_component(path).with_context(|| format!("'{path}' is not a safe relative path"))?;
    if RESERVED_ROOTS.contains(&first.as_str()) {
        bail!(proto::error::ErrorInfo::SyncTargetInvalid {
            path: path.to_string(),
            reason: proto::error::SyncReason::ManagedDir
        });
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use proto::sync::LinkState;
    use proto::warning::NotSharedReason;

    use super::*;

    fn temp_dir(tag: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("hestia-sync-{tag}-"))
            .tempdir()
            .expect("temp dir")
    }

    fn pass(name: &str, data_dir: &Path) -> Pass {
        Pass {
            id: name.to_string(),
            name: name.to_string(),
            data_dir: data_dir.to_path_buf(),
            scope: Scope::Shared,
        }
    }

    fn scoped(name: &str, data_dir: &Path, scope: Scope) -> Pass {
        Pass {
            scope,
            ..pass(name, data_dir)
        }
    }

    /// Stamped a known distance in the past, so a pass that stamps `now` where
    /// it should have stamped nothing is unambiguous.
    fn write_at(path: &Path, contents: &str, seconds_ago: u64) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
        let when = SystemTime::now() - Duration::from_secs(seconds_ago);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(when)
            .unwrap();
    }

    #[test]
    fn validate_rejects_managed_dirs_and_escapes() {
        assert!(validate_target("mods/sodium.jar").is_err());
        assert!(validate_target("resourcepacks/x").is_err());
        assert!(validate_target(".baselines/cozy").is_err());
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
        let sync = Sync::new(base.path().join("shared"));

        let mut targets = SyncTargets::default();
        targets.folders.insert("saves".to_string());
        assert!(sync.set_targets(targets).is_ok());

        let mut targets = SyncTargets::default();
        targets.files.insert("saves".to_string());
        assert!(sync.set_targets(targets).is_err());
    }

    #[test]
    fn seeds_a_new_instance_from_the_store() {
        let base = temp_dir("seed");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("options.txt"), "guiScale:3\n").unwrap();

        Sync::new(shared).apply(&pass("test", &data));

        let seeded = fs::read_to_string(data.join("options.txt")).unwrap();
        assert!(seeded.contains("guiScale:3"));
    }

    #[test]
    fn pack_selection_stays_entry_local() {
        let base = temp_dir("packs");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(
            data.join("options.txt"),
            "guiScale:2\nresourcePacks:[\"cozy\"]\n",
        )
        .unwrap();

        Sync::new(shared.clone()).apply(&pass("test", &data));

        let stored = fs::read_to_string(shared.join("options.txt")).unwrap();
        assert!(stored.contains("guiScale:2"));
        assert!(
            !stored.contains("resourcePacks"),
            "pack selection must not propagate to the shared store"
        );
        // The entry keeps its own pack selection.
        let local = fs::read_to_string(data.join("options.txt")).unwrap();
        assert!(local.contains("resourcePacks"));
    }

    /// The drift the baseline exists to stop: an instance that changed nothing
    /// must not outrank one that did, however the clock falls.
    #[test]
    fn an_idle_instance_cannot_revert_another_ones_edit() {
        let base = temp_dir("drift");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = Sync::new(shared.clone());

        write_at(&shared.join("servers.dat"), "one", 300);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));
        assert_eq!(fs::read_to_string(a.join("servers.dat")).unwrap(), "one");

        // b adds a server in game; a then launches, having changed nothing.
        write_at(&b.join("servers.dat"), "one+two", 100);
        sync.apply(&pass("a", &a));

        // b's edit survives its own next pass, and reaches the store.
        sync.apply(&pass("b", &b));
        assert_eq!(
            fs::read_to_string(b.join("servers.dat")).unwrap(),
            "one+two"
        );
        assert_eq!(
            fs::read_to_string(shared.join("servers.dat")).unwrap(),
            "one+two"
        );

        // And a picks it up at its next launch.
        sync.apply(&pass("a", &a));
        assert_eq!(
            fs::read_to_string(a.join("servers.dat")).unwrap(),
            "one+two"
        );
    }

    #[test]
    fn an_idle_instance_cannot_revert_an_options_change() {
        let base = temp_dir("optdrift");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = Sync::new(shared.clone());

        write_at(&shared.join("options.txt"), "guiScale:1\n", 300);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));

        write_at(&b.join("options.txt"), "guiScale:4\n", 100);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));

        assert!(fs::read_to_string(b.join("options.txt"))
            .unwrap()
            .contains("guiScale:4"));
        assert!(fs::read_to_string(shared.join("options.txt"))
            .unwrap()
            .contains("guiScale:4"));
    }

    /// Two instances editing different settings both survive — the reason
    /// `options.txt` is merged by key rather than copied whole.
    #[test]
    fn two_instances_changing_different_keys_both_survive() {
        let base = temp_dir("optkeys");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = Sync::new(shared.clone());

        write_at(&shared.join("options.txt"), "guiScale:1\nfov:70\n", 300);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));

        write_at(&a.join("options.txt"), "guiScale:3\nfov:70\n", 200);
        write_at(&b.join("options.txt"), "guiScale:1\nfov:90\n", 100);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));
        sync.apply(&pass("a", &a));

        let merged = fs::read_to_string(a.join("options.txt")).unwrap();
        assert!(
            merged.contains("guiScale:3"),
            "a's change survives: {merged}"
        );
        assert!(merged.contains("fov:90"), "b's change reaches a: {merged}");
    }

    #[test]
    fn a_missing_file_is_restored_rather_than_deleted_from_the_store() {
        let base = temp_dir("nodelete");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        let sync = Sync::new(shared.clone());

        write_at(&shared.join("servers.dat"), "one", 300);
        sync.apply(&pass("test", &data));
        fs::remove_file(data.join("servers.dat")).unwrap();
        sync.apply(&pass("test", &data));

        assert_eq!(
            fs::read_to_string(shared.join("servers.dat")).unwrap(),
            "one"
        );
        assert_eq!(fs::read_to_string(data.join("servers.dat")).unwrap(), "one");
    }

    #[test]
    fn forget_drops_only_that_instances_baselines() {
        let base = temp_dir("forget");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = Sync::new(shared.clone());

        write_at(&shared.join("servers.dat"), "one", 300);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));
        assert!(shared.join(BASELINES).join("a").exists());

        sync.forget("a");
        assert!(!shared.join(BASELINES).join("a").exists());
        assert!(shared.join(BASELINES).join("b").exists());
    }

    #[test]
    fn a_session_replays_the_scope_it_launched_under() {
        let base = temp_dir("session");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        let store = base
            .path()
            .join("instance")
            .join("profiles")
            .join("showcase");
        let sync = Sync::new(shared.clone());
        sync.capture(&store).unwrap();

        let launched = scoped("test", &data, Scope::Profile(store.clone()));
        sync.apply(&launched);
        sync.remember("instance-test-1", launched);

        // The game wrote settings during the session; the exit pass must file
        // them under the captured store, not whichever is active now.
        fs::write(data.join("options.txt"), "guiScale:5\n").unwrap();
        let recalled = sync.recall("instance-test-1").expect("recorded at launch");
        sync.apply(&recalled);

        assert!(fs::read_to_string(store.join("options.txt"))
            .unwrap()
            .contains("guiScale:5"));
        assert!(!shared.join("options.txt").exists());
        assert!(sync.recall("instance-test-1").is_none());
    }

    #[test]
    fn leaving_sharing_copies_the_shared_folders_out() {
        let base = temp_dir("detach");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        let sync = Sync::new(shared.clone());
        let pass = pass("test", &data);

        sync.apply(&pass);
        fs::create_dir_all(shared.join("saves").join("world")).unwrap();
        fs::write(shared.join("saves").join("world").join("level.dat"), "w").unwrap();

        let warnings = sync.detach(&pass).unwrap();

        assert!(
            link::read_target(&data.join("saves")).is_none(),
            "no longer a link"
        );
        assert_eq!(
            fs::read_to_string(data.join("saves").join("world").join("level.dat")).unwrap(),
            "w",
            "it plays what it played before"
        );
        assert!(
            shared.join("saves").join("world").is_dir(),
            "the store is untouched"
        );
        assert!(warnings.iter().any(
            |w| matches!(w, WarningInfo::SyncTargetDuplicated { target, .. } if target == "saves")
        ));

        // What it does from here is its own.
        fs::write(data.join("saves").join("world").join("level.dat"), "mine").unwrap();
        assert_eq!(
            fs::read_to_string(shared.join("saves").join("world").join("level.dat")).unwrap(),
            "w"
        );
        assert!(!shared.join(BASELINES).join("test").exists());
    }

    #[test]
    fn rejoining_lets_the_store_win_a_clash() {
        let base = temp_dir("attach");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(shared.join("saves").join("world")).unwrap();
        fs::write(shared.join("saves").join("world").join("level.dat"), "ours").unwrap();
        write_at(&shared.join("servers.dat"), "shared", 300);
        fs::create_dir_all(data.join("saves").join("world")).unwrap();
        fs::write(data.join("saves").join("world").join("level.dat"), "mine").unwrap();
        fs::create_dir_all(data.join("saves").join("solo")).unwrap();
        write_at(&data.join("servers.dat"), "mine", 100);

        let sync = Sync::new(shared.clone());
        let pass = pass("test", &data);
        let warnings = sync.attach(&pass).unwrap();

        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));
        assert_eq!(
            fs::read_to_string(shared.join("saves").join("world").join("level.dat")).unwrap(),
            "ours",
            "the store's copy survives the clash"
        );
        assert!(
            shared.join("saves").join("solo").is_dir(),
            "what only this instance had comes with it"
        );
        assert!(warnings.iter().any(|w| matches!(
            w,
            WarningInfo::SyncEntriesReplaced { entries, .. } if entries == &vec!["world".to_string()]
        )));
        // And the store wins a copied file too, newer though the instance's is.
        assert_eq!(
            fs::read_to_string(data.join("servers.dat")).unwrap(),
            "shared"
        );
    }

    #[test]
    fn apply_links_missing_and_empty_folders() {
        let base = temp_dir("linkfresh");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(data.join("config")).unwrap();

        Sync::new(shared.clone()).apply(&pass("test", &data));

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
        let data2 = base.path().join("data2");
        fs::create_dir_all(&data2).unwrap();
        Sync::new(shared.clone()).apply(&pass("other", &data2));
        assert!(data2.join("saves").join("world").is_dir());
    }

    #[test]
    fn apply_adopts_a_folder_the_store_cannot_clash_with() {
        let base = temp_dir("adopt-apply");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(data.join("saves").join("old-world")).unwrap();
        fs::write(data.join("saves").join("old-world").join("level.dat"), "x").unwrap();

        let sync = Sync::new(shared.clone());
        let warnings = sync.apply(&pass("test", &data));

        assert!(
            warnings.is_empty(),
            "the user did nothing to be warned about"
        );
        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));
        assert_eq!(
            fs::read_to_string(shared.join("saves").join("old-world").join("level.dat")).unwrap(),
            "x",
            "the world moved into the store"
        );
        // And still opens where the game looks for it.
        assert!(data
            .join("saves")
            .join("old-world")
            .join("level.dat")
            .exists());
    }

    #[test]
    fn apply_leaves_a_folder_that_would_overwrite_the_store() {
        let base = temp_dir("guard");
        let shared = base.path().join("shared");
        fs::create_dir_all(shared.join("saves").join("old-world")).unwrap();
        fs::write(
            shared.join("saves").join("old-world").join("level.dat"),
            "store",
        )
        .unwrap();
        let data = base.path().join("data");
        fs::create_dir_all(data.join("saves").join("old-world")).unwrap();
        fs::write(
            data.join("saves").join("old-world").join("level.dat"),
            "mine",
        )
        .unwrap();

        let sync = Sync::new(shared.clone());
        let warnings = sync.apply(&pass("test", &data));

        assert!(link::read_target(&data.join("saves")).is_none());
        assert_eq!(
            fs::read_to_string(data.join("saves").join("old-world").join("level.dat")).unwrap(),
            "mine"
        );
        assert_eq!(
            fs::read_to_string(shared.join("saves").join("old-world").join("level.dat")).unwrap(),
            "store"
        );
        assert!(warnings.iter().any(|w| matches!(
            w,
            WarningInfo::SyncTargetNotShared {
                reason: NotSharedReason::Collides,
                ..
            }
        )));
        let states = sync.status(&data);
        let saves = states.iter().find(|t| t.target == "saves").unwrap();
        assert_eq!(saves.state, LinkState::CannotLink);
    }

    #[test]
    fn a_pack_owned_config_stays_local_until_adopted() {
        let base = temp_dir("packlocal");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(data.join("config")).unwrap();
        fs::write(data.join("config").join("pack.toml"), "x=1").unwrap();

        let sync = Sync::new(shared.clone());
        let warnings = sync.apply(&scoped("test", &data, Scope::Local));

        assert!(link::read_target(&data.join("config")).is_none());
        assert!(warnings.is_empty(), "keeping it local is not a degradation");
        // Worlds are not settings — they share as usual.
        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));

        // Adopting is the opt-in, and the link is honoured from then on.
        sync.adopt(&data, &["config".to_string()]).unwrap();
        sync.apply(&scoped("test", &data, Scope::Local));
        assert!(link::is_linked_to(
            &shared.join("config"),
            &data.join("config")
        ));
        assert!(shared.join("config").join("pack.toml").is_file());
    }

    #[test]
    fn apply_relinks_a_stale_store_link() {
        let base = temp_dir("stale");
        let old_shared = base.path().join("old-home").join("shared");
        fs::create_dir_all(old_shared.join("saves")).unwrap();
        let data = base.path().join("data");
        fs::create_dir_all(&data).unwrap();
        link::link_dir(&old_shared.join("saves"), &data.join("saves")).unwrap();

        let shared = base.path().join("new-home").join("shared");
        Sync::new(shared.clone()).apply(&pass("test", &data));

        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));
    }

    #[test]
    fn apply_leaves_a_foreign_link_alone() {
        let base = temp_dir("foreign");
        let shared = base.path().join("shared");
        let elsewhere = base.path().join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        let data = base.path().join("data");
        fs::create_dir_all(&data).unwrap();
        link::link_dir(&elsewhere, &data.join("saves")).unwrap();

        Sync::new(shared.clone()).apply(&pass("test", &data));

        assert_eq!(link::read_target(&data.join("saves")), Some(elsewhere));
    }

    #[test]
    fn adopt_moves_entries_and_links() {
        let base = temp_dir("adopt");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
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
    }

    #[test]
    fn adopt_refuses_the_whole_target_on_collision() {
        let base = temp_dir("collide");
        let shared = base.path().join("shared");
        fs::create_dir_all(shared.join("saves").join("world")).unwrap();
        fs::write(
            shared.join("saves").join("world").join("level.dat"),
            "store",
        )
        .unwrap();
        let data = base.path().join("data");
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
    }

    #[test]
    fn capture_seeds_the_profile_store_from_the_global_one() {
        let base = temp_dir("capture");
        let shared = base.path().join("shared");
        fs::create_dir_all(shared.join("config")).unwrap();
        fs::write(shared.join("config").join("mod.toml"), "x=1").unwrap();
        fs::write(shared.join("options.txt"), "guiScale:3\n").unwrap();

        let store = base
            .path()
            .join("instance")
            .join("profiles")
            .join("showcase");
        Sync::new(shared.clone()).capture(&store).unwrap();

        assert_eq!(
            fs::read_to_string(store.join("config").join("mod.toml")).unwrap(),
            "x=1"
        );
        assert!(store.join("options.txt").is_file());
    }

    #[test]
    fn apply_scopes_settings_targets_to_the_profile_store() {
        let base = temp_dir("scoped");
        let shared = base.path().join("shared");
        fs::create_dir_all(&shared).unwrap();
        let store = base
            .path()
            .join("instance")
            .join("profiles")
            .join("showcase");
        let data = base.path().join("data");
        fs::create_dir_all(&data).unwrap();

        let sync = Sync::new(shared.clone());
        sync.capture(&store).unwrap();
        sync.apply(&scoped("test", &data, Scope::Profile(store.clone())));

        // config links into the captured store; saves stays on the global one.
        assert!(link::is_linked_to(
            &store.join("config"),
            &data.join("config")
        ));
        assert!(link::is_linked_to(
            &shared.join("saves"),
            &data.join("saves")
        ));

        // An in-game settings change lands in the captured store, not the
        // global one (the link writes through).
        fs::write(data.join("config").join("mod.toml"), "render=far").unwrap();
        assert!(store.join("config").join("mod.toml").is_file());
        assert!(!shared.join("config").join("mod.toml").exists());

        // options.txt reconciles against the captured store.
        fs::write(data.join("options.txt"), "guiScale:2\n").unwrap();
        sync.apply(&scoped("test", &data, Scope::Profile(store.clone())));
        assert!(fs::read_to_string(store.join("options.txt"))
            .unwrap()
            .contains("guiScale:2"));
        assert!(!shared.join("options.txt").exists());

        // Release, and the next un-profiled apply relinks the global store —
        // the stale captured-store link counts as a hestia store target.
        sync.release(&store).unwrap();
        assert!(!store.exists());
        sync.apply(&pass("test", &data));
        assert!(link::is_linked_to(
            &shared.join("config"),
            &data.join("config")
        ));
    }
}
