//! Persistent Minecraft instance (client) store: each instance lives at
//! `<dir>/<id>/` — its record beside the game directory the client writes into
//! (saves, options). Files shared across instances (client jars, libraries,
//! assets) live in the engine-wide stores and are materialised at launch.

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::minecraft::InstanceProfile;
use serde::{Deserialize, Serialize};

use crate::minecraft::launch::{JavaSettings, JVM_ARGS_KEY, MEMORY_KEY};
use crate::registry;

const RECORD: &str = "instance.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceRecord {
    pub id: String,
    pub name: String,
    pub created_unix: i64,
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

    pub fn instance_dir(&self, id: &str) -> PathBuf {
        self.dir().join(id)
    }

    pub fn list(&self) -> Vec<InstanceRecord> {
        let mut records: Vec<InstanceRecord> = registry::scan(&self.dir(), RECORD);
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    /// Find one instance by id or name.
    pub fn get(&self, reference: &str) -> Option<InstanceRecord> {
        self.list()
            .into_iter()
            .find(|r| r.id == reference || r.name == reference)
    }

    pub fn create(&self, name: &str, profile: InstanceProfile) -> Result<InstanceRecord> {
        let id = registry::slugify(name)?;
        if self.get(&id).is_some() || self.get(name).is_some() {
            bail!("an instance named '{name}' already exists");
        }
        let record = InstanceRecord {
            id: id.clone(),
            name: name.to_string(),
            created_unix: registry::now_unix(),
            jvm: JavaSettings::default(),
            profile,
        };
        let dir = self.instance_dir(&id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create {}", dir.display()))?;
        registry::write_record(&dir, RECORD, &record)?;
        tracing::info!(id, name, "instance registered");
        Ok(record)
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
        registry::write_record(&self.instance_dir(&record.id), RECORD, &record)
    }

    /// Both JVM settings with their current values (empty when unset).
    pub fn config_list(&self, id: &str) -> Result<Vec<(String, String)>> {
        let record = self
            .get(id)
            .with_context(|| format!("unknown instance: {id}"))?;
        Ok(record.jvm.entries())
    }

    /// Delete an instance's directory (record, saves and all). Returns false
    /// when no instance matches.
    pub fn remove(&self, reference: &str) -> Result<bool> {
        let Some(record) = self.get(reference) else {
            return Ok(false);
        };
        let dir = self.instance_dir(&record.id);
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("cannot remove {}", dir.display()))?;
        tracing::info!(id = %record.id, "instance removed");
        Ok(true)
    }
}
