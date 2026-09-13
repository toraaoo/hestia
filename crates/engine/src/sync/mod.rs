//! Shared settings across instances: a catalogue of units, the shared copy of
//! each under `<data_home>/shared/`, and the reconcile a launch runs.

mod catalogue;
mod history;
mod options;
mod reconcile;
mod servers;
mod state;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use proto::sync::{SyncConfig, SyncOption, SyncOverrides, SyncUnit, UnitState, UnitStatus};
use proto::warning::WarningInfo;

use state::Catalogue;

/// Agreements live inside the store they describe, so a captured profile
/// carries its own and `release` takes them with it.
const BASELINES: &str = ".baselines";

#[derive(Clone, Default, PartialEq, Eq)]
pub enum Scope {
    #[default]
    Shared,
    Profile(PathBuf),
}

#[derive(Clone)]
pub struct Pass {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub data_dir: PathBuf,
    pub scope: Scope,
    pub overrides: SyncOverrides,
}

pub struct Sync {
    dir: Mutex<PathBuf>,
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

    pub fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    pub fn config(&self) -> SyncConfig {
        let shared = self.dir();
        Catalogue::load(&shared).to_config(&shared)
    }

    pub fn enabled(&self, unit: SyncUnit) -> bool {
        Catalogue::load(&self.dir()).enabled(unit)
    }

    pub fn enable(&self, unit: SyncUnit, seeded_from: &str) -> Result<SyncConfig> {
        let shared = self.dir();
        let mut catalogue = Catalogue::load(&shared);
        catalogue.enable(unit, seeded_from);
        catalogue.save(&shared)?;
        Ok(catalogue.to_config(&shared))
    }

    pub fn disable(&self, unit: SyncUnit) -> Result<SyncConfig> {
        let shared = self.dir();
        let mut catalogue = Catalogue::load(&shared);
        catalogue.disable(unit);
        catalogue.save(&shared)?;
        Ok(catalogue.to_config(&shared))
    }

    pub fn set_unsynced(&self, keys: BTreeSet<String>) -> Result<SyncConfig> {
        let shared = self.dir();
        let mut catalogue = Catalogue::load(&shared);
        catalogue.set_unsynced(keys);
        catalogue.save(&shared)?;
        Ok(catalogue.to_config(&shared))
    }

    pub fn seed(&self, unit: SyncUnit, from: &Path) -> Result<()> {
        let shared = self.dir();
        std::fs::create_dir_all(&shared)
            .with_context(|| format!("cannot create {}", shared.display()))?;
        let file = catalogue::file(unit);
        let source = from.join(file);
        if source.is_file() {
            reconcile::copy_file(&source, &shared.join(file))?;
        }
        Ok(())
    }

    pub fn defer(&self, unit: SyncUnit, id: &str, data_dir: &Path) -> Result<()> {
        let file = catalogue::file(unit);
        reconcile::defer_to_store(
            &self.dir().join(BASELINES).join(id).join(file),
            &data_dir.join(file),
        )
    }

    pub fn source_state(&self, unit: SyncUnit, data_dir: &Path) -> (bool, Option<i64>) {
        let path = data_dir.join(catalogue::file(unit));
        let modified = reconcile::mtime(&path)
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|since| since.as_secs() as i64);
        (path.is_file(), modified)
    }

    /// Best-effort per unit: refusing to launch over one unreadable file would
    /// be worse than launching unshared, so a skip is returned rather than
    /// raised.
    pub fn apply(&self, pass: &Pass) -> Vec<WarningInfo> {
        let shared = self.dir();
        let catalogue = Catalogue::load(&shared);
        let mut warnings = Vec::new();
        for &unit in catalogue::ALL {
            if !catalogue.enabled(unit) || !pass.overrides.shares(unit) {
                continue;
            }
            if catalogue::era_bound(unit) && !catalogue::shares_era_bound(&pass.game_version) {
                tracing::info!(instance = %pass.name, version = %pass.game_version, %unit, "sync skipped an era-bound unit");
                warnings.push(WarningInfo::SyncUnitEraBound {
                    instance: pass.name.clone(),
                    unit,
                });
                continue;
            }
            if let Err(e) = self.settle(unit, pass, &shared, &catalogue) {
                let detail = format!("{e:#}");
                tracing::warn!(%unit, error = %detail, "sync skipped a unit");
                warnings.push(WarningInfo::SyncUnitSkipped { unit, detail });
            }
        }
        warnings
    }

    fn settle(
        &self,
        unit: SyncUnit,
        pass: &Pass,
        shared: &Path,
        catalogue: &Catalogue,
    ) -> Result<()> {
        let store = self.store_root(unit, pass, shared);
        let baselines = store.join(BASELINES).join(&pass.id);
        std::fs::create_dir_all(&baselines)
            .with_context(|| format!("cannot create {}", baselines.display()))?;
        let file = catalogue::file(unit);
        match unit {
            SyncUnit::Options => options::merge(
                &baselines.join(file),
                &store.join(file),
                &pass.data_dir.join(file),
                &excluded_keys(catalogue, pass),
            ),
            SyncUnit::Servers => servers::merge(&baselines, &store, &pass.data_dir),
            SyncUnit::Commands => history::merge(
                &baselines.join(file),
                &store.join(file),
                &pass.data_dir.join(file),
            ),
            SyncUnit::Hotbars => reconcile::whole(
                &baselines.join(file),
                &store.join(file),
                &pass.data_dir.join(file),
            ),
        }
    }

    pub fn status(&self, pass: &Pass) -> Vec<UnitStatus> {
        let shared = self.dir();
        let catalogue = Catalogue::load(&shared);
        catalogue::ALL
            .iter()
            .map(|&unit| UnitStatus {
                unit,
                state: self.state(unit, pass, &shared, &catalogue),
            })
            .collect()
    }

    fn state(
        &self,
        unit: SyncUnit,
        pass: &Pass,
        shared: &Path,
        catalogue: &Catalogue,
    ) -> UnitState {
        if !catalogue.enabled(unit) {
            return UnitState::Off;
        }
        if !pass.overrides.shares(unit) {
            return UnitState::Overridden;
        }
        if catalogue::era_bound(unit) && !catalogue::shares_era_bound(&pass.game_version) {
            return UnitState::EraBound;
        }
        let agreed = self
            .store_root(unit, pass, shared)
            .join(BASELINES)
            .join(&pass.id)
            .join(catalogue::file(unit));
        match agreed.exists() {
            true => UnitState::Synced,
            false => UnitState::Pending,
        }
    }

    pub fn options(&self) -> Vec<SyncOption> {
        let shared = self.dir();
        let catalogue = Catalogue::load(&shared);
        options::read(&shared.join(catalogue::file(SyncUnit::Options)))
            .into_iter()
            .map(|(key, value)| SyncOption {
                synced: !catalogue.unsynced().contains(&key),
                key,
                value,
            })
            .collect()
    }

    pub fn set_option(&self, key: &str, value: &str) -> Result<()> {
        let path = self.dir().join(catalogue::file(SyncUnit::Options));
        let mut values = options::read(&path);
        values.insert(key.to_string(), value.to_string());
        options::write(&path, &values)
    }

    pub fn remember(&self, session: &str, pass: Pass) {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.to_string(), pass);
    }

    pub fn recall(&self, session: &str) -> Option<Pass> {
        self.sessions.lock().unwrap().remove(session)
    }

    pub fn forget(&self, id: &str) {
        let dir = self.dir().join(BASELINES).join(id);
        if dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(&dir) {
                tracing::warn!(instance = id, error = %e, "cannot drop the sync baselines");
            }
        }
    }

    pub fn capture(&self, profile_store: &Path) -> Result<()> {
        let shared = self.dir();
        std::fs::create_dir_all(profile_store)
            .with_context(|| format!("cannot create {}", profile_store.display()))?;
        for &unit in catalogue::ALL
            .iter()
            .filter(|unit| catalogue::captured(**unit))
        {
            let file = catalogue::file(unit);
            let source = shared.join(file);
            if source.is_file() {
                reconcile::copy_file(&source, &profile_store.join(file))?;
            }
        }
        Ok(())
    }

    pub fn release(&self, profile_store: &Path) -> Result<()> {
        if profile_store.symlink_metadata().is_ok() {
            std::fs::remove_dir_all(profile_store)
                .with_context(|| format!("cannot remove {}", profile_store.display()))?;
        }
        Ok(())
    }

    fn store_root(&self, unit: SyncUnit, pass: &Pass, shared: &Path) -> PathBuf {
        match &pass.scope {
            Scope::Profile(store) if catalogue::captured(unit) => store.clone(),
            _ => shared.to_path_buf(),
        }
    }
}

fn excluded_keys(catalogue: &Catalogue, pass: &Pass) -> BTreeSet<String> {
    catalogue
        .unsynced()
        .iter()
        .cloned()
        .chain(pass.overrides.unsynced.iter().cloned())
        .chain(
            catalogue::ALWAYS_LOCAL_KEYS
                .iter()
                .map(|key| key.to_string()),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime};

    use super::*;

    fn temp_dir(tag: &str) -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix(&format!("hestia-sync-{tag}-"))
            .tempdir()
            .expect("temp dir")
    }

    fn sharing(shared: &Path) -> Sync {
        let sync = Sync::new(shared.to_path_buf());
        for &unit in catalogue::ALL {
            sync.enable(unit, "test").unwrap();
        }
        sync
    }

    fn pass(name: &str, data_dir: &Path) -> Pass {
        Pass {
            id: name.to_string(),
            name: name.to_string(),
            game_version: "1.21.4".to_string(),
            data_dir: data_dir.to_path_buf(),
            scope: Scope::Shared,
            overrides: SyncOverrides::default(),
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
    fn a_unit_is_off_until_it_is_enabled() {
        let base = temp_dir("off");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("options.txt"), "guiScale:3\n").unwrap();

        let sync = Sync::new(shared.clone());
        sync.apply(&pass("test", &data));
        assert!(!data.join("options.txt").exists());

        sync.enable(SyncUnit::Options, "other").unwrap();
        sync.apply(&pass("test", &data));
        assert!(fs::read_to_string(data.join("options.txt"))
            .unwrap()
            .contains("guiScale:3"));
    }

    #[test]
    fn seeding_starts_the_shared_copy_from_one_instance() {
        let base = temp_dir("seed");
        let shared = base.path().join("shared");
        let source = base.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("options.txt"), "fov:90\n").unwrap();

        Sync::new(shared.clone())
            .seed(SyncUnit::Options, &source)
            .unwrap();

        assert!(fs::read_to_string(shared.join("options.txt"))
            .unwrap()
            .contains("fov:90"));
    }

    #[test]
    fn deferring_lets_the_shared_copy_win() {
        let base = temp_dir("defer");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        write_at(&shared.join("options.txt"), "guiScale:1\n", 300);
        write_at(&data.join("options.txt"), "guiScale:4\n", 100);

        let sync = sharing(&shared);
        sync.defer(SyncUnit::Options, "test", &data).unwrap();
        sync.apply(&pass("test", &data));

        assert!(fs::read_to_string(data.join("options.txt"))
            .unwrap()
            .contains("guiScale:1"));
    }

    /// The drift the baseline exists to stop: an instance that changed nothing
    /// must not outrank one that did, however the clock falls.
    #[test]
    fn an_idle_instance_cannot_revert_another_ones_edit() {
        let base = temp_dir("drift");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = sharing(&shared);

        write_at(&shared.join("options.txt"), "guiScale:1\n", 300);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));

        write_at(&b.join("options.txt"), "guiScale:4\n", 100);
        sync.apply(&pass("a", &a));
        sync.apply(&pass("b", &b));
        sync.apply(&pass("a", &a));

        assert!(fs::read_to_string(a.join("options.txt"))
            .unwrap()
            .contains("guiScale:4"));
    }

    #[test]
    fn two_instances_changing_different_keys_both_survive() {
        let base = temp_dir("keys");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = sharing(&shared);

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
    fn a_key_one_instance_pins_stays_out_of_its_copy_only() {
        let base = temp_dir("pin");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = sharing(&shared);

        write_at(&shared.join("options.txt"), "guiScale:1\n", 300);
        write_at(&a.join("options.txt"), "guiScale:4\n", 100);

        let mut pinned = pass("a", &a);
        pinned.overrides.unsynced.insert("guiScale".to_string());
        sync.apply(&pinned);
        sync.apply(&pass("b", &b));

        assert!(fs::read_to_string(a.join("options.txt"))
            .unwrap()
            .contains("guiScale:4"));
        assert!(
            fs::read_to_string(shared.join("options.txt"))
                .unwrap()
                .contains("guiScale:1"),
            "pinning on one instance must not strip the key for the others"
        );
        assert!(fs::read_to_string(b.join("options.txt"))
            .unwrap()
            .contains("guiScale:1"));
    }

    #[test]
    fn pack_selection_stays_instance_local() {
        let base = temp_dir("packs");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(
            data.join("options.txt"),
            "guiScale:2\nresourcePacks:[\"cozy\"]\n",
        )
        .unwrap();

        sharing(&shared).apply(&pass("test", &data));

        let stored = fs::read_to_string(shared.join("options.txt")).unwrap();
        assert!(stored.contains("guiScale:2"));
        assert!(!stored.contains("resourcePacks"));
        assert!(fs::read_to_string(data.join("options.txt"))
            .unwrap()
            .contains("resourcePacks"));
    }

    #[test]
    fn an_instance_that_excludes_a_unit_keeps_its_own() {
        let base = temp_dir("excluded");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        write_at(&shared.join("options.txt"), "guiScale:1\n", 300);
        write_at(&data.join("options.txt"), "guiScale:4\n", 100);

        let sync = sharing(&shared);
        let mut opted_out = pass("test", &data);
        opted_out.overrides.excluded.insert(SyncUnit::Options);
        sync.apply(&opted_out);

        assert!(fs::read_to_string(data.join("options.txt"))
            .unwrap()
            .contains("guiScale:4"));
        assert_eq!(
            sync.status(&opted_out)
                .iter()
                .find(|status| status.unit == SyncUnit::Options)
                .map(|status| status.state),
            Some(UnitState::Overridden)
        );
    }

    #[test]
    fn the_command_history_is_a_union_of_both_sides() {
        let base = temp_dir("history");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        write_at(
            &shared.join("command_history.txt"),
            "/gamemode creative\n",
            300,
        );
        write_at(&data.join("command_history.txt"), "/time set day\n", 100);

        sharing(&shared).apply(&pass("test", &data));

        let merged = fs::read_to_string(data.join("command_history.txt")).unwrap();
        assert!(merged.contains("/gamemode creative"));
        assert!(merged.contains("/time set day"));
    }

    #[test]
    fn a_missing_file_is_restored_rather_than_deleted_from_the_store() {
        let base = temp_dir("nodelete");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        let sync = sharing(&shared);

        write_at(&shared.join("hotbar.nbt"), "one", 300);
        sync.apply(&pass("test", &data));
        fs::remove_file(data.join("hotbar.nbt")).unwrap();
        sync.apply(&pass("test", &data));

        assert_eq!(
            fs::read_to_string(shared.join("hotbar.nbt")).unwrap(),
            "one"
        );
        assert_eq!(fs::read_to_string(data.join("hotbar.nbt")).unwrap(), "one");
    }

    #[test]
    fn a_legacy_instance_shares_everything_but_the_options() {
        let base = temp_dir("era");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("options.txt"), "guiScale:3\n").unwrap();
        fs::write(shared.join("hotbar.nbt"), "bar").unwrap();

        let legacy = Pass {
            game_version: "1.12.2".to_string(),
            ..pass("old", &data)
        };
        let warnings = sharing(&shared).apply(&legacy);

        assert!(!data.join("options.txt").exists());
        assert!(data.join("hotbar.nbt").exists());
        assert!(warnings
            .iter()
            .any(|warning| matches!(warning, WarningInfo::SyncUnitEraBound { .. })));
    }

    #[test]
    fn the_era_boundary_is_the_1_13_format_break() {
        assert!(!catalogue::shares_era_bound("1.12.2"));
        assert!(catalogue::shares_era_bound("1.13"));
        assert!(catalogue::shares_era_bound("23w14a"));
    }

    #[test]
    fn forget_drops_only_that_instances_agreements() {
        let base = temp_dir("forget");
        let shared = base.path().join("shared");
        let a = base.path().join("a");
        let b = base.path().join("b");
        let sync = sharing(&shared);

        write_at(&shared.join("options.txt"), "guiScale:1\n", 300);
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
        let sync = sharing(&shared);
        sync.capture(&store).unwrap();

        let launched = Pass {
            scope: Scope::Profile(store.clone()),
            ..pass("test", &data)
        };
        sync.apply(&launched);
        sync.remember("instance-test-1", launched);

        fs::create_dir_all(&data).unwrap();
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
    fn editing_the_shared_options_reaches_the_next_launch() {
        let base = temp_dir("edit");
        let shared = base.path().join("shared");
        let data = base.path().join("data");
        let sync = sharing(&shared);

        sync.set_option("renderDistance", "16").unwrap();
        sync.apply(&pass("test", &data));

        assert!(fs::read_to_string(data.join("options.txt"))
            .unwrap()
            .contains("renderDistance:16"));
        assert!(sync
            .options()
            .iter()
            .any(|option| option.key == "renderDistance" && option.synced));
    }
}
