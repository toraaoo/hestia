//! Server provisioning, in-place version updates, the launch plan, and the rcon
//! console command.

use anyhow::{Context, Result};
use proto::backup::BackupKind;
use proto::minecraft::ProvisionPhase;

use proto::server::{ServerDetails, ServerPingResult};

use super::{effective_name, guard_downgrade, phase_progress};
use crate::content::install;
use crate::engine::{Engine, ServerCreateSpec, ServerUpdateSpec};
use crate::minecraft::launch::LaunchPlan;
use crate::minecraft::materialize::OnProgress;
use crate::minecraft::{ping, rcon};
use crate::servers::ServerRecord;
use crate::usage;

impl Engine {
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
        if self.servers.data_dir(&record).is_dir() {
            on_progress(&phase_progress(ProvisionPhase::Backup));
            self.backup_server(&record.id, BackupKind::Update, false, on_progress)
                .await
                .context("pre-update backup failed")?;
        }
        let java = self.ensure_java(profile.java_major, on_progress).await?;
        self.servers
            .update(&record.id, profile, Some(&self.cache), &java, on_progress)
            .await
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
        install::sync(
            &self.servers.server_dir(&record),
            &self.servers.data_dir(&record),
            None,
        )?;
        let java = self.installed_java(record.profile.java_major)?;
        let jvm = record
            .jvm
            .or_defaults(&self.config.settings().java_defaults());
        let plan = self.servers.launch_plan(&record, &java, &jvm);
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

    pub async fn server_ping(&self, reference: &str) -> Result<ServerPingResult> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        let port = record
            .game_port
            .context("this server has no game port allocated")?;
        ping::ping(port).await
    }

    pub fn server_disk_usage(&self, reference: &str) -> Result<u64> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        Ok(usage::dir_size(&self.servers.server_dir(&record)))
    }

    /// The server's static, informational view: descriptor, locations, and the
    /// on-disk footprint (a directory walk).
    pub fn server_detail(&self, reference: &str) -> Result<ServerDetails> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        let entry_dir = self.servers.server_dir(&record);
        let data_dir = self.servers.data_dir(&record);
        Ok(ServerDetails {
            id: record.id,
            name: record.name,
            flavor: record.profile.flavor,
            game_version: record.profile.game_version,
            loader_version: record.profile.loader_version,
            java_major: record.profile.java_major,
            created_unix: record.created_unix,
            game_port: record.game_port,
            disk_bytes: usage::dir_size(&entry_dir),
            entry_dir: entry_dir.to_string_lossy().into_owned(),
            data_dir: data_dir.to_string_lossy().into_owned(),
        })
    }
}
