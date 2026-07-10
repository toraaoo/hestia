//! Per-entry content management: install from a platform project, a source page
//! URL, or a local file; list, remove, and update what is installed. The managed
//! directory under the entry root is the source of truth; `data/` holds a mirror.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::content::{
    ContentAddSpec, ContentKind, ContentProject, DependencyKind, InstalledContent, SideSupport,
    VersionQuery,
};
use proto::minecraft::ProvisionPhase;

use super::phase_progress;
use crate::content::install;
use crate::engine::Engine;
use crate::instances::InstanceRecord;
use crate::minecraft::materialize::{self, OnProgress};
use crate::registry;
use crate::servers::ServerRecord;

impl Engine {
    /// Install content into a server (mods and datapacks) from a platform
    /// project, a direct URL, or a local file. Returns everything installed —
    /// the item plus any required dependencies.
    pub async fn add_server_content(
        &self,
        reference: &str,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        if !matches!(spec.kind, ContentKind::Mod | ContentKind::DataPack) {
            bail!("a server takes mods and datapacks only");
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
        let world = datapack_world(ctx, spec)?;

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
            self.add_platform_content(ctx, &resolved, &world, on_progress)
                .await?
        } else if !spec.project.is_empty() {
            self.add_platform_content(ctx, spec, &world, on_progress)
                .await?
        } else {
            vec![add_file_content(ctx, spec, &world)?]
        };

        let mut index = install::load(&ctx.entry_dir);
        for item in &items {
            let replaced = index.iter().position(|i| {
                i.kind == item.kind
                    && (i.kind != ContentKind::DataPack || i.world == item.world)
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
        world: &str,
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
                self.install_version_file(ctx, &project, &version, world, on_progress)
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
        world: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<InstalledContent> {
        let file = install::primary_file(version)?;
        materialize::validate_filename(&file.artifact.filename)?;
        let (managed, data) = content_targets(ctx, project.kind, world, &file.artifact.filename)?;
        materialize::ensure_artifact(
            Some(&self.cache),
            &file.artifact,
            &managed,
            ProvisionPhase::Content,
            on_progress,
        )
        .await?;
        if managed != data {
            install::mirror(&managed, &data)?;
        }
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
            world: world.to_string(),
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
                .install_version_file(ctx, &project, &version, &item.world, on_progress)
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
                match index.iter_mut().find(|i| {
                    i.kind == new_item.kind
                        && i.project_id == new_item.project_id
                        && (i.kind != ContentKind::DataPack || i.world == new_item.world)
                }) {
                    Some(entry) => *entry = new_item.clone(),
                    None => index.push(new_item.clone()),
                }
            }
            install::save(&ctx.entry_dir, index)?;
        }
        Ok(updated)
    }
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
/// (`Unknown` passes — the platform did not say). Datapacks are exempt: they
/// run on the server side of any world, including a client's integrated server,
/// so a source's client-side flag must not block installing one on an instance.
fn side_gate(project: &ContentProject, side: EntrySide) -> Result<()> {
    if project.kind == ContentKind::DataPack {
        return Ok(());
    }
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

/// Resolve the data-relative world directory a datapack installs into — a
/// server's single `level-name` world, or an instance's chosen (and validated)
/// save. Empty for every non-datapack kind, which install into a flat dir.
fn datapack_world(ctx: &EntryContent, spec: &ContentAddSpec) -> Result<String> {
    if spec.kind != ContentKind::DataPack {
        return Ok(String::new());
    }
    match ctx.side {
        EntrySide::Server => Ok(crate::servers::level_name(&ctx.data_dir)),
        EntrySide::Client => {
            let requested = spec.world.trim();
            if requested.is_empty() {
                bail!("name a save world for the datapack (the instance's --world)");
            }
            if !ctx.data_dir.join("saves").join(requested).is_dir() {
                bail!("no save world '{requested}' in this instance");
            }
            Ok(format!("saves/{requested}"))
        }
    }
}

/// The `(managed, data)` paths a kind's file occupies. Mods/resourcepacks/
/// shaders keep a managed copy in the entry root that is mirrored into `data/`;
/// a datapack has one file, inside its world under `data/` (managed == data),
/// so the caller skips the mirror.
fn content_targets(
    ctx: &EntryContent,
    kind: ContentKind,
    world: &str,
    filename: &str,
) -> Result<(PathBuf, PathBuf)> {
    if kind == ContentKind::DataPack {
        let path = ctx.data_dir.join(world).join("datapacks").join(filename);
        return Ok((path.clone(), path));
    }
    let dir = install::kind_dir(kind)?;
    Ok((
        ctx.entry_dir.join(dir).join(filename),
        ctx.data_dir.join(dir).join(filename),
    ))
}

/// Import a local file: copy it into the managed directory (or, for a datapack,
/// straight into its world) and mirror it into the game directory. No
/// provenance beyond the hash, so it can never update.
fn add_file_content(
    ctx: &EntryContent,
    spec: &ContentAddSpec,
    world: &str,
) -> Result<InstalledContent> {
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
    let (managed, data) = content_targets(ctx, spec.kind, world, &filename)?;
    if let Some(parent) = managed.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    std::fs::copy(source, &managed)
        .with_context(|| format!("cannot import {}", source.display()))?;
    if managed != data {
        install::mirror(&managed, &data)?;
    }
    Ok(InstalledContent {
        kind: spec.kind,
        source: "file".to_string(),
        title: filename.clone(),
        sha1: install::sha1_file(&managed)?,
        filename,
        installed_unix: registry::now_unix(),
        world: world.to_string(),
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
    let (removed, kept): (Vec<_>, Vec<_>) = install::load(&ctx.entry_dir)
        .into_iter()
        .partition(|i| i.kind == kind && install::matches(i, reference));
    if removed.is_empty() {
        return Ok(false);
    }
    for item in &removed {
        install::remove_files(&ctx.entry_dir, &ctx.data_dir, item);
        tracing::info!(
            entry = %ctx.entry_dir.display(),
            kind = ?item.kind,
            title = %item.title,
            filename = %item.filename,
            world = %item.world,
            "content removed"
        );
    }
    install::save(&ctx.entry_dir, kept)?;
    Ok(true)
}
