//! Persistent Minecraft instance (client) store: each instance lives at
//! `<dir>/<slug>/` (the name slugged; the id is opaque) — the `instance.json`
//! record beside `data/`, the game
//! directory the client writes into (saves, options). The root is reserved
//! for managed content directories (`mods/`, `resourcepacks/`, `configs/`);
//! every directory appears on demand. Files shared across
//! instances (client jars, libraries, assets) live in the engine-wide stores
//! and are materialised at launch.

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::minecraft::InstanceProfile;
use serde::{Deserialize, Serialize};

use crate::minecraft::launch::{JavaSettings, JVM_ARGS_KEY, MEMORY_KEY};
use crate::registry;

const RECORD: &str = "instance.json";
const DATA: &str = "data";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRecord {
    pub id: String,
    pub name: String,
    pub created_unix: i64,
    /// Unix time of the most recent launch; `None` until first played.
    #[serde(default)]
    pub last_played_unix: Option<i64>,
    /// Cumulative seconds played, accumulated as each session exits.
    #[serde(default)]
    pub playtime_seconds: i64,
    /// Per-entry JVM tuning (memory, extra flags) injected at each launch.
    #[serde(default)]
    pub jvm: JavaSettings,
    pub profile: InstanceProfile,
}

pub struct Instances {
    dir: Mutex<PathBuf>,
}

impl Instances {
    pub fn new(dir: PathBuf) -> Self {
        Instances {
            dir: Mutex::new(dir),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    pub fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    /// The instance's directory, named for its current display name, so a
    /// rename moves it; the id stays the entry's stable internal key.
    pub fn instance_dir(&self, record: &InstanceRecord) -> PathBuf {
        self.dir()
            .join(registry::dir_name(&record.id, &record.name))
    }

    /// The instance's game directory — everything the client itself reads and
    /// writes (saves, options, logs).
    pub fn data_dir(&self, record: &InstanceRecord) -> PathBuf {
        self.instance_dir(record).join(DATA)
    }

    pub fn list(&self) -> Vec<InstanceRecord> {
        let mut records: Vec<InstanceRecord> = registry::scan(&self.dir(), RECORD);
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    /// Find one instance by id or name (any spelling that slugs the same).
    pub fn get(&self, reference: &str) -> Option<InstanceRecord> {
        self.list()
            .into_iter()
            .find(|r| proto::naming::reference_matches(reference, &r.id, &r.name))
    }

    pub fn create(&self, name: &str, profile: InstanceProfile) -> Result<InstanceRecord> {
        if registry::name_taken(name, self.list().iter().map(|r| r.name.as_str())) {
            bail!("an instance named '{name}' already exists");
        }
        registry::slugify(name)?;
        let id = registry::allocate_id(|id| self.get(id).is_some())?;
        let record = InstanceRecord {
            id,
            name: name.to_string(),
            created_unix: registry::now_unix(),
            last_played_unix: None,
            playtime_seconds: 0,
            jvm: JavaSettings::default(),
            profile,
        };
        let dir = self.instance_dir(&record);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create {}", dir.display()))?;
        registry::write_record(&dir, RECORD, &record)?;
        tracing::info!(id = %record.id, name, "instance registered");
        Ok(record)
    }

    /// Swap the record onto a freshly resolved profile; the new version's files
    /// (version-keyed under the shared roots) materialise at the next launch.
    /// Name, JVM settings, and the game directory are untouched.
    pub fn update(&self, id: &str, profile: InstanceProfile) -> Result<InstanceRecord> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        record.profile = profile;
        registry::write_record(&self.instance_dir(&record), RECORD, &record)?;
        tracing::info!(
            id = %record.id,
            version = %record.profile.game_version,
            loader = ?record.profile.loader_version,
            "instance updated"
        );
        Ok(record)
    }

    /// Stamp the most-recent-launch time. Called when a session spawns, so the
    /// next launch no longer counts as a first play.
    pub fn mark_launched(&self, id: &str) -> Result<()> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        record.last_played_unix = Some(registry::now_unix());
        registry::write_record(&self.instance_dir(&record), RECORD, &record)
    }

    /// Add an exited session's duration to the cumulative playtime. A
    /// non-positive duration is a no-op.
    pub fn add_playtime(&self, id: &str, seconds: i64) -> Result<()> {
        if seconds <= 0 {
            return Ok(());
        }
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        record.playtime_seconds += seconds;
        registry::write_record(&self.instance_dir(&record), RECORD, &record)
    }

    /// Read one JVM setting (`memory` / `jvm-args`); `Ok(None)` means unset. An
    /// unknown key is an error naming the valid keys.
    pub fn config_get(&self, id: &str, key: &str) -> Result<Option<String>> {
        let record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        record.jvm.get(key).with_context(|| {
            format!("unknown key '{key}' (valid keys: {MEMORY_KEY}, {JVM_ARGS_KEY})")
        })
    }

    /// Write one JVM setting; an empty value clears it. Settings take effect on
    /// the next launch.
    pub fn config_set(&self, id: &str, key: &str, value: &str) -> Result<()> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        if !record.jvm.set(key, value)? {
            bail!("unknown key '{key}' (valid keys: {MEMORY_KEY}, {JVM_ARGS_KEY})");
        }
        registry::write_record(&self.instance_dir(&record), RECORD, &record)
    }

    /// Both JVM settings with their current values (empty when unset).
    pub fn config_list(&self, id: &str) -> Result<Vec<(String, String)>> {
        let record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        Ok(record.jvm.entries())
    }

    /// Rename an instance: rewrite the display name and move its directory to
    /// the new slug. The id is stable, so JVM settings and game data are
    /// untouched. The caller guarantees the instance is stopped and not busy.
    pub fn rename(&self, reference: &str, new_name: &str) -> Result<InstanceRecord> {
        let mut record = self
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        if registry::name_taken(
            new_name,
            self.list()
                .iter()
                .filter(|r| r.id != record.id)
                .map(|r| r.name.as_str()),
        ) {
            bail!("an instance named '{new_name}' already exists");
        }
        registry::slugify(new_name)?;
        let old_dir = self.instance_dir(&record);
        record.name = new_name.to_string();
        let new_dir = self.instance_dir(&record);
        if new_dir != old_dir && old_dir.exists() {
            std::fs::rename(&old_dir, &new_dir).with_context(|| {
                format!("cannot move {} to {}", old_dir.display(), new_dir.display())
            })?;
        }
        registry::write_record(&new_dir, RECORD, &record)?;
        tracing::info!(id = %record.id, name = %new_name, "instance renamed");
        Ok(record)
    }

    /// Delete an instance's directory (record, saves and all). Returns false
    /// when no instance matches.
    pub fn remove(&self, reference: &str) -> Result<bool> {
        let Some(record) = self.get(reference) else {
            return Ok(false);
        };
        let dir = self.instance_dir(&record);
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("cannot remove {}", dir.display()))?;
        tracing::info!(id = %record.id, "instance removed");
        Ok(true)
    }
}
