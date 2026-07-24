//! Instance creation, in-place version updates, and the launch preparation that
//! materialises the client jar, libraries, and assets.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use proto::instance::InstanceDetails;
use proto::minecraft::{ConfigEntry, ProvisionPhase};

use super::{effective_name, guard_downgrade};
use crate::content::{install, profiles};
use crate::engine::Engine;
use crate::instances::InstanceRecord;
use crate::minecraft::launch::{self, InstancePaths, LaunchAccount, LaunchPlan};
use crate::minecraft::log4j;
use crate::minecraft::materialize::{self, OnProgress};

impl Engine {
    /// The instance's save-world folder names (sorted) under `data/saves/` —
    /// the worlds a datapack can install into.
    pub fn instance_worlds(&self, reference: &str) -> Result<Vec<String>> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let saves = self.instances.data_dir(&record).join("saves");
        let mut worlds: Vec<String> = std::fs::read_dir(&saves)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        worlds.sort();
        Ok(worlds)
    }

    pub fn instance_disk_usage(&self, reference: &str) -> Result<u64> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        Ok(crate::usage::dir_size(
            &self.instances.instance_dir(&record),
        ))
    }

    /// The instance's static, informational view: descriptor, locations, and
    /// the on-disk footprint (a directory walk).
    pub fn instance_detail(&self, reference: &str) -> Result<InstanceDetails> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let entry_dir = self.instances.instance_dir(&record);
        let data_dir = self.instances.data_dir(&record);
        Ok(InstanceDetails {
            id: record.id,
            name: record.name,
            flavor: record.profile.flavor,
            game_version: record.profile.game_version,
            loader_version: record.profile.loader_version,
            java_major: record.profile.java_major,
            created_unix: record.created_unix,
            last_played_unix: record.last_played_unix,
            playtime_seconds: record.playtime_seconds,
            disk_bytes: crate::usage::dir_size(&entry_dir),
            entry_dir: entry_dir.to_string_lossy().into_owned(),
            data_dir: data_dir.to_string_lossy().into_owned(),
        })
    }

    /// Move an instance to another version of its flavor. A downgrade must be
    /// allowed explicitly — Minecraft cannot load saves written by a newer
    /// version, and **nothing is backed up first** (instances have no backup
    /// story until import/export lands). Only the record changes; files
    /// materialise at the next launch.
    pub async fn update_instance(
        &self,
        reference: &str,
        version: &str,
        loader_version: Option<String>,
        allow_downgrade: bool,
    ) -> Result<InstanceRecord> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let versions = self
            .minecraft
            .instance_versions(&record.profile.flavor)
            .await?;
        guard_downgrade(
            "saves",
            &record.name,
            &record.profile.game_version,
            version,
            &versions,
            allow_downgrade,
        )?;
        let profile = self
            .minecraft
            .resolve_instance(&record.profile.flavor, version, loader_version)
            .await?;
        self.instances.update(&record.id, profile)
    }

    /// Create an instance record from a freshly resolved profile; its files are
    /// materialised by `prepare_instance` at launch time.
    pub async fn create_instance(
        &self,
        name: &str,
        flavor: &str,
        version: &str,
        loader_version: Option<String>,
        config: &[ConfigEntry],
    ) -> Result<InstanceRecord> {
        let profile = self
            .minecraft
            .resolve_instance(flavor, version, loader_version)
            .await?;
        let name = effective_name(name, flavor, version);
        let record = self.instances.create(&name, profile)?;

        let applied = config.iter().try_for_each(|entry| {
            self.instances
                .config_set(&record.id, &entry.key, &entry.value)
        });
        if let Err(e) = applied {
            let _ = self.instances.remove(&record.id);
            return Err(e);
        }
        self.instances
            .get(&record.id)
            .with_context(|| format!("instance '{}' vanished after create", record.id))
    }

    /// Materialise everything an instance launch needs — the Java runtime, the
    /// client jar, libraries, assets — and assemble the JVM invocation for the
    /// given account (empty picks the sole signed-in one). `profile` overrides
    /// the active profile for this launch (`none` = no profile); `reconcile`
    /// off skips the sync/mirror pass entirely — sessions are already running,
    /// so the mirror is in use (jars are open, locked on Windows).
    pub async fn prepare_instance(
        &self,
        reference: &str,
        account: &str,
        session_seq: u32,
        profile: &str,
        reconcile: bool,
        on_progress: OnProgress<'_>,
    ) -> Result<(InstanceRecord, LaunchPlan, PathBuf)> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let entry_dir = self.instances.instance_dir(&record);
        let launch_profile = if reconcile {
            profiles::resolve(&entry_dir, profile)?
        } else {
            None
        };
        let account = self.launch_account(account).await?;

        let java = self
            .ensure_java(record.profile.java_major, on_progress)
            .await?;

        materialize::validate_filename(&record.profile.game_version)?;
        let meta = meta_dir(&self.data_home());
        let client_jar = meta
            .join("versions")
            .join(&record.profile.game_version)
            .join("client.jar");
        materialize::ensure_artifact(
            Some(&self.cache),
            &record.profile.client,
            &client_jar,
            ProvisionPhase::Client,
            on_progress,
        )
        .await?;

        let libraries_root = meta.join("libraries");
        materialize::ensure_libraries(
            Some(&self.cache),
            &record.profile.libraries,
            &libraries_root,
            on_progress,
        )
        .await?;

        let assets_root = meta.join("assets");
        materialize::ensure_assets(
            Some(&self.cache),
            &record.profile.asset_index,
            &assets_root,
            on_progress,
        )
        .await?;

        let game_dir = self.instances.data_dir(&record);
        std::fs::create_dir_all(&game_dir)
            .with_context(|| format!("cannot create {}", game_dir.display()))?;
        if reconcile {
            // A captured profile scopes the settings-class sync targets to its
            // own store; an uncaptured one inherits the global store.
            let store = launch_profile
                .as_ref()
                .filter(|p| p.captured)
                .map(|p| profiles::store_dir(&entry_dir, &p.name));
            self.sync.apply(&game_dir, store.as_deref())?;
            let selection: Option<std::collections::HashSet<String>> =
                launch_profile.map(|p| p.members.into_iter().collect());
            install::sync(&entry_dir, &game_dir, selection.as_ref())?;
        }
        let natives_dir = meta.join("natives").join(&record.profile.game_version);
        std::fs::create_dir_all(&natives_dir)
            .with_context(|| format!("cannot create {}", natives_dir.display()))?;

        // Per-session logging lives under the instance root (not data/, so it is
        // outside backups): each concurrent session gets its own file the
        // supervisor can tail independently.
        let session_dir = self.instances.instance_dir(&record).join("logs");
        std::fs::create_dir_all(&session_dir)
            .with_context(|| format!("cannot create {}", session_dir.display()))?;
        let log_file = session_dir.join(format!("session-{session_seq}.log"));
        let log_config = session_dir.join(format!("session-{session_seq}.xml"));
        std::fs::write(&log_config, log4j::session_config(&log_file))
            .with_context(|| format!("cannot write {}", log_config.display()))?;

        let jvm = record
            .jvm
            .or_defaults(&self.config.settings().java_defaults());
        let plan = launch::instance_plan(
            &record.profile,
            &java,
            &InstancePaths {
                game_dir: &game_dir,
                natives_dir: &natives_dir,
                client_jar: &client_jar,
                libraries_root: &libraries_root,
                assets_root: &assets_root,
                log_config: Some(&log_config),
            },
            &account,
            &jvm,
        );
        Ok((record, plan, log_file))
    }

    async fn launch_account(&self, reference: &str) -> Result<LaunchAccount> {
        let account = if reference.is_empty() {
            self.accounts
                .default_account()
                .context("no Minecraft account is signed in (run `hestia account login`)")?
        } else {
            self.accounts
                .list()
                .into_iter()
                .find(|a| a.name.eq_ignore_ascii_case(reference) || a.uuid == reference)
                .with_context(|| format!("no account matches '{reference}'"))?
        };
        let access_token = self.accounts.access_token(&account.uuid).await?;
        Ok(LaunchAccount {
            name: account.name,
            uuid: account.uuid,
            access_token,
        })
    }
}

/// The root for launcher-managed shared game files (versions, libraries,
/// assets, natives) — the Modrinth layout, keeping the data home itself to
/// user-facing entries and launcher internals.
fn meta_dir(home: &Path) -> PathBuf {
    home.join("meta")
}
