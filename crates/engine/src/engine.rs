//! The daemon-internal aggregate root; front-ends reach it only over IPC. Adding
//! a domain = a module, a member, and a getter here.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use proto::backup::{BackupInfo, BackupKind};
use proto::content::{
    ContentAddSpec, ContentKind, ContentProject, DependencyKind, InstalledContent, SideSupport,
    VersionQuery,
};
use proto::minecraft::{ConfigEntry, ProvisionPhase, ProvisionProgress};

use crate::accounts::Accounts;
use crate::backup;
use crate::cache::Cache;
use crate::config::Config;
use crate::content::{install, Content};
use crate::instances::{InstanceRecord, Instances};
use crate::java::Java;
use crate::minecraft::launch::{self, InstancePaths, LaunchAccount, LaunchPlan};
use crate::minecraft::materialize::{self, OnProgress};
use crate::minecraft::rcon;
use crate::minecraft::Minecraft;
use crate::registry;
use crate::servers::{RconConfig, ServerRecord, Servers};

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
    content: Content,
    servers: Servers,
    instances: Instances,
    // One backup or restore per entry at a time: two archives of the same
    // data would interleave the rcon save-off/save-on dance.
    backups_active: Mutex<HashSet<String>>,
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
            content: Content::new(),
            servers,
            instances,
            backups_active: Mutex::new(HashSet::new()),
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

    pub fn content(&self) -> &Content {
        &self.content
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
        if self.servers.data_dir(&record.id).is_dir() {
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
        if self.instances.data_dir(&record.id).is_dir() {
            self.backup_instance(&record.id, BackupKind::Update, &|_| {})
                .await
                .context("pre-update backup failed")?;
        }
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
        install::sync(
            &self.servers.server_dir(&record.id),
            &self.servers.data_dir(&record.id),
        )?;
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

    /// Archive a server's `data/` into its `backups/`. With `live` (the
    /// caller observed the server running) world saving pauses over RCON
    /// around the archive — save-off, save-all flush, tar, save-on — and
    /// always resumes, even when archiving fails (the docker-mc-backup dance).
    pub async fn backup_server(
        &self,
        reference: &str,
        kind: BackupKind,
        live: bool,
        on_progress: OnProgress<'_>,
    ) -> Result<BackupInfo> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        if !record.ready {
            bail!("server '{}' is still provisioning", record.name);
        }
        let _claim = self.claim_backup(format!("server-{}", record.id))?;
        let paused = if live {
            Some(self.pause_world_saves(&record).await?)
        } else {
            None
        };
        let result = run_backup(
            self.servers.server_dir(&record.id),
            self.servers.data_dir(&record.id),
            kind,
            server_backup_excludes(&record),
            on_progress,
        )
        .await;
        if let Some(rcon) = paused {
            if let Err(e) = resume_world_saves(&rcon).await {
                tracing::error!(
                    server = %record.id,
                    error = format!("{e:#}"),
                    "world saving is still disabled"
                );
                if result.is_ok() {
                    return Err(e.context(
                        "the backup was created, but world saving could not be re-enabled \
                         (run `save-on` in the server console)",
                    ));
                }
            }
        }
        result
    }

    /// Replace a stopped server's `data/` with a backup's content. The jar and
    /// libraries of the record's current version carry over — a backup holds
    /// the world and configuration, not the re-materialisable binaries.
    pub async fn restore_server_backup(
        &self,
        reference: &str,
        backup: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<BackupInfo> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        if !record.ready {
            bail!("server '{}' is still provisioning", record.name);
        }
        let _claim = self.claim_backup(format!("server-{}", record.id))?;
        run_restore(
            self.servers.server_dir(&record.id),
            self.servers.data_dir(&record.id),
            backup.to_string(),
            server_backup_excludes(&record),
            on_progress,
        )
        .await
    }

    /// A server's stored backups, newest first.
    pub fn server_backups(&self, reference: &str) -> Result<Vec<BackupInfo>> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        Ok(backup::list(&self.servers.server_dir(&record.id)))
    }

    /// Delete one server backup. Returns false when no backup matches.
    pub fn remove_server_backup(&self, reference: &str, backup: &str) -> Result<bool> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        backup::remove(&self.servers.server_dir(&record.id), backup)
    }

    /// Prune a server's *scheduled* backups beyond its retention (manual and
    /// pre-update backups are kept until removed explicitly). Returns what was
    /// removed.
    pub fn prune_server_backups(&self, reference: &str) -> Result<Vec<BackupInfo>> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        backup::prune(
            &self.servers.server_dir(&record.id),
            BackupKind::Scheduled,
            record.backup.retention(),
        )
    }

    /// Archive a stopped instance's `data/` into its `backups/`.
    pub async fn backup_instance(
        &self,
        reference: &str,
        kind: BackupKind,
        on_progress: OnProgress<'_>,
    ) -> Result<BackupInfo> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let _claim = self.claim_backup(format!("instance-{}", record.id))?;
        run_backup(
            self.instances.instance_dir(&record.id),
            self.instances.data_dir(&record.id),
            kind,
            instance_backup_excludes(),
            on_progress,
        )
        .await
    }

    /// Replace a stopped instance's `data/` with a backup's content.
    pub async fn restore_instance_backup(
        &self,
        reference: &str,
        backup: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<BackupInfo> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let _claim = self.claim_backup(format!("instance-{}", record.id))?;
        run_restore(
            self.instances.instance_dir(&record.id),
            self.instances.data_dir(&record.id),
            backup.to_string(),
            instance_backup_excludes(),
            on_progress,
        )
        .await
    }

    /// An instance's stored backups, newest first.
    pub fn instance_backups(&self, reference: &str) -> Result<Vec<BackupInfo>> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        Ok(backup::list(&self.instances.instance_dir(&record.id)))
    }

    /// Delete one instance backup. Returns false when no backup matches.
    pub fn remove_instance_backup(&self, reference: &str, backup: &str) -> Result<bool> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        backup::remove(&self.instances.instance_dir(&record.id), backup)
    }

    /// Install content into a server (mods only) from a platform project, a
    /// direct URL, or a local file. Returns everything installed — the item
    /// plus any required dependencies.
    pub async fn add_server_content(
        &self,
        reference: &str,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        if spec.kind != ContentKind::Mod {
            bail!("a server takes mods only");
        }
        let (_, ctx) = self.server_content_ctx(reference)?;
        self.add_content(&ctx, spec, on_progress).await
    }

    /// Install content into an instance (mods, resourcepacks, shaders).
    pub async fn add_instance_content(
        &self,
        reference: &str,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        install::kind_dir(spec.kind)?;
        let (_, ctx) = self.instance_content_ctx(reference)?;
        self.add_content(&ctx, spec, on_progress).await
    }

    /// A server's installed items of one kind, plus untracked filenames found
    /// in its game directory.
    pub fn server_content(
        &self,
        reference: &str,
        kind: ContentKind,
    ) -> Result<(Vec<InstalledContent>, Vec<String>)> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        Ok(list_content(&ctx, kind))
    }

    pub fn instance_content(
        &self,
        reference: &str,
        kind: ContentKind,
    ) -> Result<(Vec<InstalledContent>, Vec<String>)> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        Ok(list_content(&ctx, kind))
    }

    /// Uninstall one item (matched by project id, slug, filename, or title).
    /// Returns false when nothing matches.
    pub fn remove_server_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
    ) -> Result<bool> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        remove_content(&ctx, kind, item)
    }

    pub fn remove_instance_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
    ) -> Result<bool> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        remove_content(&ctx, kind, item)
    }

    /// Move platform-sourced items to their newest compatible version — one
    /// named item, or every item of the kind when `item` is empty. Returns
    /// what actually changed.
    pub async fn update_server_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        self.update_content(&ctx, kind, item, on_progress).await
    }

    pub async fn update_instance_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        self.update_content(&ctx, kind, item, on_progress).await
    }

    fn server_content_ctx(&self, reference: &str) -> Result<(ServerRecord, EntryContent)> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        if !record.ready {
            bail!("server '{}' is still provisioning", record.name);
        }
        let ctx = EntryContent {
            entry_dir: self.servers.server_dir(&record.id),
            data_dir: self.servers.data_dir(&record.id),
            game_version: record.profile.game_version.clone(),
            flavor: record.profile.flavor.clone(),
            side: EntrySide::Server,
        };
        Ok((record, ctx))
    }

    fn instance_content_ctx(&self, reference: &str) -> Result<(InstanceRecord, EntryContent)> {
        let record = self
            .instances
            .get(reference)
            .with_context(|| format!("unknown instance: {reference}"))?;
        let ctx = EntryContent {
            entry_dir: self.instances.instance_dir(&record.id),
            data_dir: self.instances.data_dir(&record.id),
            game_version: record.profile.game_version.clone(),
            flavor: record.profile.flavor.clone(),
            side: EntrySide::Client,
        };
        Ok((record, ctx))
    }

    async fn add_content(
        &self,
        ctx: &EntryContent,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        install::kind_dir(spec.kind)?;
        if spec.kind == ContentKind::Mod && ctx.flavor == "vanilla" {
            bail!("a vanilla {} cannot load mods", ctx.side.noun());
        }
        let picked = [&spec.project, &spec.url, &spec.path]
            .iter()
            .filter(|s| !s.is_empty())
            .count();
        if picked != 1 {
            bail!("specify exactly one of a project, a url, or a file");
        }

        let items = if !spec.url.is_empty() {
            let (source, parsed) = self.content.parse_url(&spec.url).with_context(|| {
                format!(
                    "'{}' is not a project URL on a supported content source",
                    spec.url
                )
            })?;
            let mut resolved = spec.clone();
            resolved.source = source;
            resolved.project = parsed.project;
            if let Some(version) = parsed.version {
                resolved.version = version;
            }
            self.add_platform_content(ctx, &resolved, on_progress)
                .await?
        } else if !spec.project.is_empty() {
            self.add_platform_content(ctx, spec, on_progress).await?
        } else {
            vec![add_file_content(ctx, spec)?]
        };

        let mut index = install::load(&ctx.entry_dir);
        for item in &items {
            let replaced = index.iter().position(|i| {
                i.kind == item.kind
                    && ((!item.project_id.is_empty() && i.project_id == item.project_id)
                        || i.filename == item.filename)
            });
            if let Some(pos) = replaced {
                let old = index.remove(pos);
                if old.filename != item.filename {
                    install::remove_files(&ctx.entry_dir, &ctx.data_dir, &old);
                }
            }
            index.push(item.clone());
        }
        install::save(&ctx.entry_dir, index)?;
        for item in &items {
            tracing::info!(
                entry = %ctx.entry_dir.display(),
                kind = ?item.kind,
                title = %item.title,
                filename = %item.filename,
                version = %item.version_number,
                "content installed"
            );
        }
        Ok(items)
    }

    /// Resolve a platform project (and, for mods, its required dependencies —
    /// breadth-first, skipping anything already installed) and download each
    /// pick into the managed directory.
    async fn add_platform_content(
        &self,
        ctx: &EntryContent,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        on_progress(&phase_progress(ProvisionPhase::Resolving));
        let root = self.content.project(&spec.source, &spec.project).await?;
        if root.kind != spec.kind {
            bail!(
                "'{}' is {:?} content, not {:?}",
                root.title,
                root.kind,
                spec.kind
            );
        }
        side_gate(&root, ctx.side)?;

        let mut visited: std::collections::HashSet<String> = install::load(&ctx.entry_dir)
            .into_iter()
            .map(|i| i.project_id)
            .filter(|p| !p.is_empty())
            .collect();
        visited.insert(root.id.clone());
        let loader = (spec.kind == ContentKind::Mod).then(|| ctx.flavor.clone());

        let mut items = Vec::new();
        let mut queue = vec![(root, spec.version.clone())];
        while let Some((project, pin)) = queue.pop() {
            let versions = self
                .content
                .versions(&VersionQuery {
                    source: spec.source.clone(),
                    project: project.id.clone(),
                    loader: loader.clone(),
                    game_version: Some(ctx.game_version.clone()),
                })
                .await?;
            let version =
                install::pick_version(&versions, &ctx.game_version, loader.as_deref(), &pin)
                    .with_context(|| format!("cannot install '{}'", project.title))?
                    .clone();

            if spec.kind == ContentKind::Mod {
                for dep in &version.dependencies {
                    if dep.kind != DependencyKind::Required {
                        continue;
                    }
                    if dep.project_id.is_empty() {
                        tracing::warn!(
                            of = %project.title,
                            version_id = %dep.version_id,
                            "required dependency names no project; skipping"
                        );
                        continue;
                    }
                    if !visited.insert(dep.project_id.clone()) {
                        continue;
                    }
                    let dep_project = self.content.project(&spec.source, &dep.project_id).await?;
                    if side_gate(&dep_project, ctx.side).is_err() {
                        tracing::warn!(
                            dependency = %dep_project.title,
                            of = %project.title,
                            "required dependency does not support this side; skipping"
                        );
                        continue;
                    }
                    queue.push((dep_project, String::new()));
                }
            }

            items.push(
                self.install_version_file(ctx, &project, &version, on_progress)
                    .await?,
            );
        }
        Ok(items)
    }

    async fn install_version_file(
        &self,
        ctx: &EntryContent,
        project: &ContentProject,
        version: &proto::content::ContentVersion,
        on_progress: OnProgress<'_>,
    ) -> Result<InstalledContent> {
        let file = install::primary_file(version)?;
        materialize::validate_filename(&file.artifact.filename)?;
        let dir = install::kind_dir(project.kind)?;
        let managed = ctx.entry_dir.join(dir).join(&file.artifact.filename);
        materialize::ensure_artifact(
            Some(&self.cache),
            &file.artifact,
            &managed,
            ProvisionPhase::Content,
            on_progress,
        )
        .await?;
        install::mirror(
            &managed,
            &ctx.data_dir.join(dir).join(&file.artifact.filename),
        )?;
        Ok(InstalledContent {
            kind: project.kind,
            source: version.source.clone(),
            project_id: project.id.clone(),
            slug: project.slug.clone(),
            title: project.title.clone(),
            version_id: version.id.clone(),
            version_number: version.version_number.clone(),
            filename: file.artifact.filename.clone(),
            sha1: file
                .artifact
                .checksum
                .as_ref()
                .map(|c| c.hex.clone())
                .unwrap_or_default(),
            url: file.artifact.url.clone(),
            installed_unix: registry::now_unix(),
        })
    }

    async fn update_content(
        &self,
        ctx: &EntryContent,
        kind: ContentKind,
        reference: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        let index = install::load(&ctx.entry_dir);
        let targets: Vec<InstalledContent> = index
            .iter()
            .filter(|i| i.kind == kind && (reference.is_empty() || install::matches(i, reference)))
            .cloned()
            .collect();
        if targets.is_empty() {
            match reference.is_empty() {
                true => bail!("nothing is installed"),
                false => bail!("no installed item matches '{reference}'"),
            }
        }
        let loader = (kind == ContentKind::Mod).then(|| ctx.flavor.clone());

        let mut updated = Vec::new();
        for item in targets {
            if item.project_id.is_empty() {
                if !reference.is_empty() {
                    bail!(
                        "'{}' was installed from a {} and cannot be updated",
                        item.filename,
                        item.source
                    );
                }
                continue;
            }
            on_progress(&phase_progress(ProvisionPhase::Resolving));
            let versions = self
                .content
                .versions(&VersionQuery {
                    source: item.source.clone(),
                    project: item.project_id.clone(),
                    loader: loader.clone(),
                    game_version: Some(ctx.game_version.clone()),
                })
                .await?;
            let version =
                install::pick_version(&versions, &ctx.game_version, loader.as_deref(), "")
                    .with_context(|| format!("cannot update '{}'", item.title))?
                    .clone();
            if version.id == item.version_id {
                continue;
            }
            let project = ContentProject {
                id: item.project_id.clone(),
                slug: item.slug.clone(),
                title: item.title.clone(),
                kind: item.kind,
                ..ContentProject::default()
            };
            let new_item = self
                .install_version_file(ctx, &project, &version, on_progress)
                .await?;
            if new_item.filename != item.filename {
                install::remove_files(&ctx.entry_dir, &ctx.data_dir, &item);
            }
            tracing::info!(
                title = %item.title,
                from = %item.version_number,
                to = %new_item.version_number,
                "content updated"
            );
            updated.push(new_item);
        }

        if !updated.is_empty() {
            let mut index = install::load(&ctx.entry_dir);
            for new_item in &updated {
                match index
                    .iter_mut()
                    .find(|i| i.kind == new_item.kind && i.project_id == new_item.project_id)
                {
                    Some(entry) => *entry = new_item.clone(),
                    None => index.push(new_item.clone()),
                }
            }
            install::save(&ctx.entry_dir, index)?;
        }
        Ok(updated)
    }

    fn claim_backup(&self, key: String) -> Result<BackupClaim<'_>> {
        let mut active = self.backups_active.lock().unwrap();
        if !active.insert(key.clone()) {
            bail!("a backup or restore is already running for this entry");
        }
        Ok(BackupClaim {
            active: &self.backups_active,
            key,
        })
    }

    /// Pause world writes before archiving a live server: `save-off` stops
    /// autosaves, `save-all flush` forces everything pending onto disk (the
    /// reply arrives once the flush completed).
    async fn pause_world_saves(&self, record: &ServerRecord) -> Result<RconConfig> {
        let rcon_cfg = record
            .rcon
            .clone()
            .context("this server has no console yet (restart it to enable one)")?;
        let mut conn = rcon::Rcon::connect(rcon_cfg.port, &rcon_cfg.password).await?;
        conn.command("save-off").await?;
        conn.command("save-all flush").await?;
        Ok(rcon_cfg)
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
        install::sync(&self.instances.instance_dir(&record.id), &game_dir)?;
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

/// The entry-shape a content operation needs, independent of whether the entry
/// is a server or an instance.
struct EntryContent {
    entry_dir: PathBuf,
    data_dir: PathBuf,
    game_version: String,
    flavor: String,
    side: EntrySide,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntrySide {
    Server,
    Client,
}

impl EntrySide {
    fn noun(self) -> &'static str {
        match self {
            EntrySide::Server => "server",
            EntrySide::Client => "instance",
        }
    }
}

/// Reject content the platform marks unsupported for the entry's side
/// (`Unknown` passes — the platform did not say).
fn side_gate(project: &ContentProject, side: EntrySide) -> Result<()> {
    let support = match side {
        EntrySide::Server => project.server_side,
        EntrySide::Client => project.client_side,
    };
    if support == SideSupport::Unsupported {
        bail!(
            "'{}' does not support the {} side",
            project.title,
            side.noun()
        );
    }
    Ok(())
}

/// Import a local file: copy it into the managed directory and mirror it into
/// the game directory. No provenance beyond the hash, so it can never update.
fn add_file_content(ctx: &EntryContent, spec: &ContentAddSpec) -> Result<InstalledContent> {
    let source = Path::new(&spec.path);
    if !source.is_file() {
        bail!("no file at {}", source.display());
    }
    let filename = if spec.filename.is_empty() {
        source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        spec.filename.clone()
    };
    materialize::validate_filename(&filename)?;
    let dir = install::kind_dir(spec.kind)?;
    let managed = ctx.entry_dir.join(dir).join(&filename);
    if let Some(parent) = managed.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    std::fs::copy(source, &managed)
        .with_context(|| format!("cannot import {}", source.display()))?;
    install::mirror(&managed, &ctx.data_dir.join(dir).join(&filename))?;
    Ok(InstalledContent {
        kind: spec.kind,
        source: "file".to_string(),
        title: filename.clone(),
        sha1: install::sha1_file(&managed)?,
        filename,
        installed_unix: registry::now_unix(),
        ..InstalledContent::default()
    })
}

fn list_content(ctx: &EntryContent, kind: ContentKind) -> (Vec<InstalledContent>, Vec<String>) {
    let items: Vec<InstalledContent> = install::load(&ctx.entry_dir)
        .into_iter()
        .filter(|i| i.kind == kind)
        .collect();
    let untracked = install::untracked(&ctx.data_dir, kind, &items);
    (items, untracked)
}

fn remove_content(ctx: &EntryContent, kind: ContentKind, reference: &str) -> Result<bool> {
    let mut index = install::load(&ctx.entry_dir);
    let Some(pos) = index
        .iter()
        .position(|i| i.kind == kind && install::matches(i, reference))
    else {
        return Ok(false);
    };
    let removed = index.remove(pos);
    install::remove_files(&ctx.entry_dir, &ctx.data_dir, &removed);
    install::save(&ctx.entry_dir, index)?;
    tracing::info!(
        entry = %ctx.entry_dir.display(),
        kind = ?removed.kind,
        title = %removed.title,
        filename = %removed.filename,
        "content removed"
    );
    Ok(true)
}

struct BackupClaim<'a> {
    active: &'a Mutex<std::collections::HashSet<String>>,
    key: String,
}

impl Drop for BackupClaim<'_> {
    fn drop(&mut self) {
        self.active.lock().unwrap().remove(&self.key);
    }
}

/// What a server backup skips and a restore carries over: content the
/// launcher re-materialises for the record's *current* version (jar,
/// libraries) plus logs and cache — the docker-mc-backup default set — and
/// the managed content mirror (`mods/`), which the sync pass re-creates from
/// the entry root at the next start.
fn server_backup_excludes(record: &ServerRecord) -> Vec<String> {
    vec![
        record.profile.primary.filename.clone(),
        "libraries".into(),
        "logs".into(),
        "cache".into(),
        "mods".into(),
    ]
}

fn instance_backup_excludes() -> Vec<String> {
    vec![
        "logs".into(),
        "mods".into(),
        "resourcepacks".into(),
        "shaders".into(),
    ]
}

/// Run the blocking archive pass off-thread, forwarding its per-file ticks to
/// `on_progress` as `Backup`-phase provisioning progress.
async fn run_backup(
    entry_dir: PathBuf,
    data_dir: PathBuf,
    kind: BackupKind,
    exclude: Vec<String>,
    on_progress: OnProgress<'_>,
) -> Result<BackupInfo> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::task::spawn_blocking(move || {
        backup::create(
            &entry_dir,
            &data_dir,
            kind,
            &exclude,
            &move |current, total| {
                let _ = tx.send((current, total));
            },
        )
    });
    while let Some((current, total)) = rx.recv().await {
        on_progress(&backup_progress(current, total));
    }
    task.await.context("the backup task panicked")?
}

async fn run_restore(
    entry_dir: PathBuf,
    data_dir: PathBuf,
    backup: String,
    preserve: Vec<String>,
    on_progress: OnProgress<'_>,
) -> Result<BackupInfo> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::task::spawn_blocking(move || {
        backup::restore(
            &entry_dir,
            &data_dir,
            &backup,
            &preserve,
            &move |current, total| {
                let _ = tx.send((current, total));
            },
        )
    });
    while let Some((current, total)) = rx.recv().await {
        on_progress(&backup_progress(current, total));
    }
    task.await.context("the restore task panicked")?
}

fn backup_progress(current: u64, total: u64) -> ProvisionProgress {
    ProvisionProgress {
        phase: ProvisionPhase::Backup,
        current,
        total,
        detail: String::new(),
    }
}

/// `save-on` must reach the server even when archiving failed, or the world
/// stops persisting — retry like docker-mc-backup's exit trap does.
async fn resume_world_saves(rcon_cfg: &RconConfig) -> Result<()> {
    let mut last = anyhow::anyhow!("rcon unreachable");
    for attempt in 0..5 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        match rcon::Rcon::connect(rcon_cfg.port, &rcon_cfg.password).await {
            Ok(mut conn) => match conn.command("save-on").await {
                Ok(_) => return Ok(()),
                Err(e) => last = e,
            },
            Err(e) => last = e,
        }
    }
    Err(last.context("cannot re-enable world saving"))
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
