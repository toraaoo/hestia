//! Instance creation, in-place version updates, and the launch preparation that
//! materialises the client jar, libraries, and assets.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::error::{ErrorInfo, Field, Reason};
use proto::instance::{InstanceDetails, QuickPlay, WorldInfo};
use proto::minecraft::{ConfigEntry, ProvisionPhase};
use proto::warning::WarningInfo;

use super::{effective_name, guard_downgrade, meta_dir};
use crate::content::{install, profiles};
use crate::engine::Engine;
use crate::instances::InstanceRecord;
use crate::minecraft::launch::{self, InstancePaths, LaunchAccount, LaunchPlan};
use crate::minecraft::log4j;
use crate::minecraft::materialize::{self, OnProgress};
use crate::minecraft::{ping, world};

impl Engine {
    /// The instance's save worlds under `data/saves/`, each described from its
    /// own `level.dat` — the worlds a datapack can install into, and what the
    /// player calls them. Sorted by folder, since that is the stable identity.
    pub fn instance_worlds(&self, reference: &str) -> Result<Vec<WorldInfo>> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let saves = self.instances.data_dir(&record).join("saves");
        let mut folders: Vec<String> = std::fs::read_dir(&saves)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        folders.sort();
        Ok(folders
            .iter()
            .map(|folder| world::describe(&saves, folder))
            .collect())
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

    /// Delete an instance and everything the launcher kept *about* it: the
    /// directory, and its sync baselines in the shared store, which nothing
    /// else would ever collect. Returns false when no instance matches.
    pub fn remove_instance(&self, reference: &str) -> Result<bool> {
        let Some(record) = self.instances.get(reference) else {
            return Ok(false);
        };
        let removed = self.instances.remove(&record.id)?;
        self.sync.forget(&record.id);
        Ok(removed)
    }

    /// Move an instance to another version of its flavor. A downgrade must be
    /// allowed explicitly — Minecraft cannot load saves written by a newer
    /// version, and **nothing is backed up first**: an instance is kept by
    /// exporting it, which is a deliberate act rather than something an update
    /// does for you. Only the record changes; files materialise at the next
    /// launch.
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
            let _ = self.remove_instance(&record.id);
            return Err(e);
        }

        // Re-read before linking: the config entries above may have opted this
        // instance out of sharing, and the create-time record predates them.
        let record = self
            .instances
            .get(&record.id)
            .with_context(|| format!("instance '{}' vanished after create", record.id))?;
        let data_dir = self.instances.data_dir(&record);
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            tracing::warn!(id = %record.id, error = %e, "cannot create the game directory");
        } else {
            self.link_new_instance(&record, &data_dir);
        }
        Ok(record)
    }

    /// Materialise everything an instance launch needs — the Java runtime, the
    /// client jar, libraries, assets — and assemble the JVM invocation.
    ///
    /// A quick-play target is validated up front, before any of that work: a
    /// launch that cannot join what it was asked to join should fail in the
    /// moment it was asked, not after several minutes of downloads.
    pub async fn prepare_instance(
        &self,
        request: LaunchRequest<'_>,
        on_progress: OnProgress<'_>,
    ) -> Result<PreparedLaunch> {
        let LaunchRequest {
            instance: reference,
            account,
            session_seq,
            profile,
            reconcile,
            quick_play,
        } = request;
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let entry_dir = self.instances.instance_dir(&record);
        if let Some(target) = quick_play.as_ref() {
            validate_quick_play(
                &record.profile.game_version,
                &self.instances.data_dir(&record),
                target,
            )?;
        }
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
        // `versions/<id>/<id>.jar`, Mojang's own layout rather than a name of
        // our choosing: a modloader that boots off the module path filters the
        // vanilla jar out of it by that name (NeoForge passes
        // `-DignoreList=…,${version_name}.jar`). Called anything else, the jar
        // stays on the module path beside the loader's patched copy and the JVM
        // refuses to resolve — two modules exporting the same packages.
        let client_jar = meta
            .join("versions")
            .join(&record.profile.game_version)
            .join(format!("{}.jar", record.profile.game_version));
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

        self.minecraft
            .install_instance(
                &record.profile.flavor,
                &crate::minecraft::InstallRequest {
                    game_version: &record.profile.game_version,
                    loader_version: record.profile.loader_version.as_deref(),
                    root: &meta,
                    meta: &meta,
                    minecraft_jar: &client_jar,
                    java: &java,
                    cache: Some(&self.cache),
                    processes: self.processes(),
                },
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
        let mut warnings = Vec::new();
        if reconcile {
            // A captured profile scopes the settings-class sync targets to its
            // own store; an uncaptured one inherits the global store.
            let store = launch_profile
                .as_ref()
                .filter(|p| p.captured)
                .map(|p| profiles::store_dir(&entry_dir, &p.name));
            if let Some(pass) = self.instance_pass(&record, &entry_dir, &game_dir, store.as_deref())
            {
                let session = proto::naming::instance_session_id(&record.id, session_seq);
                warnings = self.begin_instance_sync(&session, pass);
            }
            let selection: Option<std::collections::HashSet<String>> =
                launch_profile.map(|p| p.members.into_iter().collect());
            let worlds = crate::instances::save_worlds(&game_dir);
            install::sync(&entry_dir, &game_dir, selection.as_ref(), &worlds)?;
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
            quick_play.as_ref(),
        );
        Ok(PreparedLaunch {
            record,
            plan,
            log_file,
            warnings,
        })
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

/// One launch's inputs: which instance, as whom, under which profile, and what
/// it joins on start. A struct rather than a parameter list — the two `&str`s
/// that mean entirely different things sit next to each other, and a caller
/// naming them cannot swap them by accident.
pub struct LaunchRequest<'a> {
    pub instance: &'a str,
    /// Account name or uuid; empty picks the sole signed-in one.
    pub account: &'a str,
    pub session_seq: u32,
    /// A profile override for this launch: empty is the active profile, the
    /// literal `none` is no profile.
    pub profile: &'a str,
    /// Off skips the sync/mirror pass entirely — other sessions are already
    /// running, so the mirror is in use (jars are open, locked on Windows).
    pub reconcile: bool,
    /// Join a world or server on start instead of opening to the title screen.
    pub quick_play: Option<QuickPlay>,
}

/// Refuse a quick-play target the launch could not honour: a client too old for
/// the arguments at all, a world folder that is not there, or an address the
/// game would not parse. Each answers as the typed refusal a front-end renders,
/// rather than as a launch that silently opens the title screen.
fn validate_quick_play(game_version: &str, data_dir: &Path, target: &QuickPlay) -> Result<()> {
    if !launch::supports_quick_play(game_version) {
        bail!(ErrorInfo::QuickPlayUnsupported {
            version: game_version.to_string(),
        });
    }
    match target {
        QuickPlay::World(folder) => {
            let folder = folder.trim();
            if !data_dir.join("saves").join(folder).is_dir() {
                bail!(ErrorInfo::WorldNotFound {
                    world: folder.to_string(),
                });
            }
        }
        QuickPlay::Server(address) => {
            if ping::split_address(address).is_err() {
                bail!(ErrorInfo::InvalidValue {
                    field: Field::Address,
                    reason: Reason::ServerAddress,
                });
            }
        }
    }
    Ok(())
}

/// Everything a launch needs to spawn, plus what went less than perfectly on
/// the way there. The warnings ride out on the result rather than staying in the
/// daemon log, so the caller can tell the user what the session is running
/// against.
pub struct PreparedLaunch {
    pub record: InstanceRecord,
    pub plan: LaunchPlan,
    pub log_file: PathBuf,
    pub warnings: Vec<WarningInfo>,
}
