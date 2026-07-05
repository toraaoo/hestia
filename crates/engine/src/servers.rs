//! Persistent Minecraft server store: each server lives at `<dir>/<id>/`
//! (its jar, `eula.txt`, and the world the game writes) beside a `server.json`
//! record; listing scans the directory — the disk is the registry.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::minecraft::{ProvisionPhase, ServerProfile};
use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::minecraft::launch::{self, LaunchPlan};
use crate::minecraft::materialize::{self, OnProgress};
use crate::registry;

const RECORD: &str = "server.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub created_unix: i64,
    /// False until the create job has finished provisioning files.
    #[serde(default)]
    pub ready: bool,
    pub profile: ServerProfile,
}

pub struct Servers {
    dir: Mutex<PathBuf>,
}

impl Servers {
    pub fn new(dir: PathBuf) -> Self {
        Servers {
            dir: Mutex::new(dir),
        }
    }

    pub fn reload(&self, dir: PathBuf) {
        *self.dir.lock().unwrap() = dir;
    }

    pub fn dir(&self) -> PathBuf {
        self.dir.lock().unwrap().clone()
    }

    pub fn server_dir(&self, id: &str) -> PathBuf {
        self.dir().join(id)
    }

    pub fn list(&self) -> Vec<ServerRecord> {
        let mut records: Vec<ServerRecord> = registry::scan(&self.dir(), RECORD);
        records.sort_by(|a, b| a.name.cmp(&b.name));
        records
    }

    /// Find one server by id or name.
    pub fn get(&self, reference: &str) -> Option<ServerRecord> {
        self.list()
            .into_iter()
            .find(|r| r.id == reference || r.name == reference)
    }

    /// Register a new server: allocate its id from the name, create its
    /// directory, and write the (not yet ready) record.
    pub fn create(&self, name: &str, profile: ServerProfile) -> Result<ServerRecord> {
        let id = registry::slugify(name)?;
        if self.get(&id).is_some() || self.get(name).is_some() {
            bail!("a server named '{name}' already exists");
        }
        let record = ServerRecord {
            id: id.clone(),
            name: name.to_string(),
            created_unix: registry::now_unix(),
            ready: false,
            profile,
        };
        let dir = self.server_dir(&id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create {}", dir.display()))?;
        registry::write_record(&dir, RECORD, &record)?;
        tracing::info!(id, name, "server registered");
        Ok(record)
    }

    /// Download the server's files into its directory and record the EULA
    /// acceptance the caller obtained from the user.
    pub async fn provision(
        &self,
        record: &ServerRecord,
        cache: Option<&Cache>,
        on_progress: OnProgress<'_>,
    ) -> Result<()> {
        let dir = self.server_dir(&record.id);
        materialize::validate_filename(&record.profile.primary.filename)?;
        if !record.profile.libraries.is_empty() {
            materialize::ensure_libraries(
                cache,
                &record.profile.libraries,
                &dir.join("libraries"),
                on_progress,
            )
            .await?;
        }
        materialize::ensure_artifact(
            cache,
            &record.profile.primary,
            &dir.join(&record.profile.primary.filename),
            ProvisionPhase::Server,
            on_progress,
        )
        .await?;
        std::fs::write(dir.join("eula.txt"), "eula=true\n").context("cannot write eula.txt")?;
        tracing::info!(id = %record.id, "server provisioned");
        Ok(())
    }

    pub fn mark_ready(&self, id: &str) -> Result<ServerRecord> {
        let mut record = self
            .get(id)
            .with_context(|| format!("unknown server: {id}"))?;
        record.ready = true;
        registry::write_record(&self.server_dir(id), RECORD, &record)?;
        Ok(record)
    }

    /// Delete a server's directory (jar, world and all). Returns false when no
    /// server matches.
    pub fn remove(&self, reference: &str) -> Result<bool> {
        let Some(record) = self.get(reference) else {
            return Ok(false);
        };
        let dir = self.server_dir(&record.id);
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("cannot remove {}", dir.display()))?;
        tracing::info!(id = %record.id, "server removed");
        Ok(true)
    }

    pub fn launch_plan(&self, record: &ServerRecord, java: &Path) -> LaunchPlan {
        launch::server_plan(&record.profile, java, &self.server_dir(&record.id))
    }
}
