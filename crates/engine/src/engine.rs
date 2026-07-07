//! The daemon-internal aggregate root; front-ends reach it only over IPC. Adding
//! a domain = a module, a member, and a getter here.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use proto::minecraft::{ConfigEntry, ProvisionPhase, ProvisionProgress};

use crate::accounts::Accounts;
use crate::cache::Cache;
use crate::config::Config;
use crate::instances::{InstanceRecord, Instances};
use crate::java::Java;
use crate::minecraft::launch::{self, InstancePaths, LaunchAccount, LaunchPlan};
use crate::minecraft::materialize::{self, OnProgress};
use crate::minecraft::rcon;
use crate::minecraft::Minecraft;
use crate::servers::{ServerRecord, Servers};

/// Everything a server create needs from the caller — the engine-side input to
/// `provision_server` (EULA assertion and job ids are daemon concerns).
#[derive(Debug, Clone, Default)]
pub struct ServerCreateSpec {
    pub name: String,
    pub flavor: String,
    pub version: String,
    pub loader_version: Option<String>,
    pub port: Option<u16>,
    pub config: Vec<ConfigEntry>,
}

/// Everything a server update needs from the caller — the engine-side input to
/// `update_server` (the downgrade confirmation is obtained by the front-end).
#[derive(Debug, Clone, Default)]
pub struct ServerUpdateSpec {
    pub server: String,
    pub version: String,
    pub loader_version: Option<String>,
    pub allow_downgrade: bool,
}

pub struct Engine {
    data_home: Mutex<PathBuf>,
    config: Config,
    cache: Cache,
    java: Java,
    accounts: Accounts,
    minecraft: Minecraft,
    servers: Servers,
    instances: Instances,
}

impl Engine {
    pub fn new(override_home: Option<&Path>) -> Self {
        let data_home = common::paths::data_home(override_home);
        tracing::info!(home = %data_home.display(), "engine data home");
        let config = Config::new(common::paths::config_path(Some(&data_home)));
        let cache = Cache::new(data_home.join("cache"));
        let java = Java::new(data_home.join("java"));
        let accounts = Accounts::new(data_home.join("accounts.json"));
        let servers = Servers::new(data_home.join("servers"));
        let instances = Instances::new(data_home.join("instances"));
        Engine {
            data_home: Mutex::new(data_home),
            config,
            cache,
            java,
            accounts,
            minecraft: Minecraft::new(),
            servers,
            instances,
        }
    }

    pub fn data_home(&self) -> PathBuf {
        self.data_home.lock().unwrap().clone()
    }

    /// Persist `dir` (empty reverts to the default), re-resolve, and repoint every
    /// subsystem on the running daemon.
    pub fn set_data_home(&self, dir: &str) -> std::io::Result<PathBuf> {
        common::paths::set_persisted_home(Path::new(dir))?;
        let resolved = common::paths::data_home(None);
        self.config
            .reload(common::paths::config_path(Some(&resolved)));
        self.cache.reload(resolved.join("cache"));
        self.java.reload(resolved.join("java"));
        self.accounts.reload(resolved.join("accounts.json"));
        self.servers.reload(resolved.join("servers"));
        self.instances.reload(resolved.join("instances"));
        *self.data_home.lock().unwrap() = resolved.clone();
        tracing::info!(home = %resolved.display(), "engine data home changed");
        Ok(resolved)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    pub fn java(&self) -> &Java {
        &self.java
    }

    pub fn accounts(&self) -> &Accounts {
        &self.accounts
    }

    pub fn minecraft(&self) -> &Minecraft {
        &self.minecraft
    }

    pub fn servers(&self) -> &Servers {
        &self.servers
    }

    pub fn instances(&self) -> &Instances {
        &self.instances
    }

    /// Create a fully provisioned server: resolve the profile, register the
    /// record, ensure the Java runtime, and download its files. A failure after
    /// registration removes the record so nothing half-built is left behind.
    /// The caller is responsible for having obtained the user's EULA acceptance.
    pub async fn provision_server(
        &self,
        spec: ServerCreateSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<ServerRecord> {
        on_progress(&phase_progress(ProvisionPhase::Resolving));
        let profile = self
            .minecraft
            .resolve_server(&spec.flavor, &spec.version, spec.loader_version)
            .await?;
        let name = effective_name(&spec.name, &spec.flavor, &spec.version);
        let record = self.servers.create(&name, profile, spec.port)?;

        // Config entries apply after provisioning so property keys validate
        // against the schema the server itself generated.
        let provisioned = async {
            let java = self
                .ensure_java(record.profile.java_major, on_progress)
                .await?;
            self.servers
                .provision(&record, Some(&self.cache), &java, on_progress)
                .await?;
            for entry in &spec.config {
                self.servers
                    .config_set(&record.id, &entry.key, &entry.value)?;
            }
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if provisioned.is_err() {
            let _ = self.servers.remove(&record.id);
        }
        provisioned?;
        self.servers.mark_ready(&record.id)
    }

    /// Move a server to another version of its flavor. A downgrade must be
    /// allowed explicitly — Minecraft cannot load a world written by a newer
    /// version.
    pub async fn update_server(
        &self,
        spec: ServerUpdateSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<ServerRecord> {
        let record = self
            .servers
            .get(&spec.server)
            .with_context(|| format!("unknown server: {}", spec.server))?;
        let versions = self
            .minecraft
            .server_versions(&record.profile.flavor)
            .await?;
        guard_downgrade(
            "world",
            &record.name,
            &record.profile.game_version,
            &spec.version,
            &versions,
            spec.allow_downgrade,
        )?;
        on_progress(&phase_progress(ProvisionPhase::Resolving));
        let profile = self
            .minecraft
            .resolve_server(&record.profile.flavor, &spec.version, spec.loader_version)
            .await?;
        let java = self.ensure_java(profile.java_major, on_progress).await?;
        self.servers
            .update(&record.id, profile, Some(&self.cache), &java, on_progress)
            .await
    }

    /// Move an instance to another version of its flavor. A downgrade must be
    /// allowed explicitly — Minecraft cannot load saves written by a newer
    /// version. Only the record changes; files materialise at the next launch.
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

    /// The ready-to-spawn invocation for a provisioned server, with its ports
    /// reconciled into `server.properties`.
    pub fn server_launch_plan(&self, reference: &str) -> Result<(ServerRecord, LaunchPlan)> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        if !record.ready {
            anyhow::bail!("server '{}' is still provisioning", record.name);
        }
        let record = self.servers.ensure_start_config(&record.id)?;
        let java = self.installed_java(record.profile.java_major)?;
        let plan = self.servers.launch_plan(&record, &java);
        Ok((record, plan))
    }

    /// Send one console command to a running server over its RCON channel and
    /// return the server's reply.
    pub async fn server_command(&self, reference: &str, command: &str) -> Result<String> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        let rcon = record
            .rcon
            .context("this server has no console yet (restart it to enable one)")?;
        let mut conn = rcon::Rcon::connect(rcon.port, &rcon.password).await?;
        conn.command(command).await
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
    /// given account (empty picks the sole signed-in one).
    pub async fn prepare_instance(
        &self,
        reference: &str,
        account: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<(InstanceRecord, LaunchPlan)> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
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

        let game_dir = self.instances.data_dir(&record.id);
        std::fs::create_dir_all(&game_dir)
            .with_context(|| format!("cannot create {}", game_dir.display()))?;
        let natives_dir = meta.join("natives").join(&record.profile.game_version);
        std::fs::create_dir_all(&natives_dir)
            .with_context(|| format!("cannot create {}", natives_dir.display()))?;

        let plan = launch::instance_plan(
            &record.profile,
            &java,
            &InstancePaths {
                game_dir: &game_dir,
                natives_dir: &natives_dir,
                client_jar: &client_jar,
                libraries_root: &libraries_root,
                assets_root: &assets_root,
            },
            &account,
            &record.jvm,
        );
        Ok((record, plan))
    }

    /// The installed runtime for `major`, installing it (through the cache) when
    /// missing.
    async fn ensure_java(&self, major: i32, on_progress: OnProgress<'_>) -> Result<PathBuf> {
        let detail = format!("java {major}");
        let outcome = self
            .java
            .install(major, false, Some(&self.cache), |jp| {
                on_progress(&ProvisionProgress {
                    phase: ProvisionPhase::Java,
                    current: jp.current,
                    total: jp.total,
                    detail: detail.clone(),
                });
            })
            .await?;
        Ok(outcome.runtime.executable)
    }

    fn installed_java(&self, major: i32) -> Result<PathBuf> {
        self.java
            .installed()
            .into_iter()
            .find(|r| r.major == major)
            .map(|r| r.executable)
            .with_context(|| {
                format!("java {major} is not installed (run `hestia java install {major}`)")
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

/// The root for launcher-managed shared game files (versions, libraries,
/// assets, natives) — the Modrinth layout, keeping the data home itself to
/// user-facing entries and launcher internals.
fn meta_dir(home: &Path) -> PathBuf {
    home.join("meta")
}

/// Reject an unconfirmed downgrade. The direction comes from the flavor's own
/// newest-first catalogue; a version the catalogue no longer lists is
/// undecidable and passes (the front-end still confirms what it can detect).
fn guard_downgrade(
    data: &str,
    name: &str,
    from: &str,
    to: &str,
    versions: &[proto::minecraft::GameVersion],
    allowed: bool,
) -> Result<()> {
    if !allowed && proto::minecraft::downgrade_between(versions, from, to) == Some(true) {
        anyhow::bail!(
            "moving '{name}' from {from} back to {to} is a downgrade, and Minecraft cannot \
             load {data} written by a newer version; confirm the downgrade to proceed"
        );
    }
    Ok(())
}

fn effective_name(name: &str, flavor: &str, version: &str) -> String {
    if name.trim().is_empty() {
        format!("{flavor}-{version}")
    } else {
        name.trim().to_string()
    }
}

fn phase_progress(phase: ProvisionPhase) -> ProvisionProgress {
    ProvisionProgress {
        phase,
        current: 0,
        total: 0,
        detail: String::new(),
    }
}
