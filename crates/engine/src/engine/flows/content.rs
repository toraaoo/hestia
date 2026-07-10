//! Per-entry content management: install from a platform project, a source page
//! URL, or a local file; list, remove, and update what is installed. The managed
//! directory under the entry root is the source of truth; `data/` holds a mirror.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use proto::content::{
    ContentAddItem, ContentAddSpec, ContentFailure, ContentKind, ContentProject, DependencyKind,
    InstalledContent, SideSupport, VersionQuery,
};
use proto::minecraft::{ProvisionPhase, ProvisionProgress};

use super::phase_progress;
use crate::content::install;
use crate::engine::Engine;
use crate::instances::InstanceRecord;
use crate::minecraft::materialize::{self, OnProgress};
use crate::registry;
use crate::servers::ServerRecord;

impl Engine {
    /// Install a batch of content into a server (mods and datapacks) — each
    /// item a platform project, a direct URL, or a local file. Returns
    /// everything installed (items plus required dependencies) and, per item
    /// that could not be installed, a failure; the batch continues past them.
    pub async fn add_server_content(
        &self,
        reference: &str,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>)> {
        if !matches!(spec.kind, ContentKind::Mod | ContentKind::DataPack) {
            bail!("a server takes mods and datapacks only");
        }
        let (_, ctx) = self.server_content_ctx(reference)?;
        self.add_content(&ctx, spec, on_progress).await
    }

    /// Install a batch of content into an instance (mods, resourcepacks,
    /// shaders, datapacks).
    pub async fn add_instance_content(
        &self,
        reference: &str,
        spec: &ContentAddSpec,
        on_progress: OnProgress<'_>,
    ) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>)> {
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
    ) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>)> {
        install::kind_dir(spec.kind)?;
        if spec.kind == ContentKind::Mod && ctx.flavor == "vanilla" {
            bail!("a vanilla {} cannot load mods", ctx.side.noun());
        }
        if spec.items.is_empty() {
            bail!("nothing to install");
        }
        let worlds = datapack_worlds(ctx, spec)?;

        let mut failures = Vec::new();
        let mut roots = Vec::new();
        let mut files = Vec::new();
        for item in &spec.items {
            let picked = [&item.project, &item.url, &item.path]
                .iter()
                .filter(|s| !s.is_empty())
                .count();
            if picked != 1 {
                failures.push(failure(
                    item_label(item),
                    "",
                    "specify exactly one of a project, a url, or a file",
                ));
                continue;
            }
            if !item.url.is_empty() {
                match self.content.parse_url(&item.url) {
                    Some((source, parsed)) => roots.push(PlatformRoot {
                        given: item.url.clone(),
                        source,
                        pin: parsed.version.unwrap_or_else(|| item.version.clone()),
                        project: parsed.project,
                    }),
                    None => failures.push(failure(
                        &item.url,
                        "",
                        format!(
                            "'{}' is not a project URL on a supported content source",
                            item.url
                        ),
                    )),
                }
            } else if !item.project.is_empty() {
                roots.push(PlatformRoot {
                    given: item.project.clone(),
                    source: spec.source.clone(),
                    project: item.project.clone(),
                    pin: item.version.clone(),
                });
            } else {
                files.push(item);
            }
        }

        let mut items = Vec::new();
        for item in files {
            match add_file_content(ctx, spec.kind, item, &worlds) {
                Ok(mut installed) => items.append(&mut installed),
                Err(e) => failures.push(failure(&item.path, "", format!("{e:#}"))),
            }
        }
        let (mut platform_items, mut platform_failures) = self
            .add_platform_content(ctx, spec.kind, roots, &worlds, on_progress)
            .await;
        items.append(&mut platform_items);
        failures.append(&mut platform_failures);

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
        for fail in &failures {
            tracing::warn!(
                entry = %ctx.entry_dir.display(),
                item = %fail.item,
                message = %fail.message,
                "content install failed"
            );
        }
        Ok((items, failures))
    }

    /// Resolve every platform root (and, for mods, required dependencies —
    /// breadth-first under one visited set, so a dependency shared across the
    /// batch installs once) and download each pick into the managed directory.
    /// A node that fails records a per-item failure and the batch continues.
    async fn add_platform_content(
        &self,
        ctx: &EntryContent,
        kind: ContentKind,
        roots: Vec<PlatformRoot>,
        worlds: &[String],
        on_progress: OnProgress<'_>,
    ) -> (Vec<InstalledContent>, Vec<ContentFailure>) {
        let mut items = Vec::new();
        let mut failures = Vec::new();
        if roots.is_empty() {
            return (items, failures);
        }
        on_progress(&phase_progress(ProvisionPhase::Resolving));

        let mut visited: HashSet<String> = install::load(&ctx.entry_dir)
            .into_iter()
            .map(|i| i.project_id)
            .filter(|p| !p.is_empty())
            .collect();
        let loader = (kind == ContentKind::Mod).then(|| ctx.flavor.clone());

        // An explicitly named root installs even when already present (a
        // reinstall/re-pin); only duplicates within the batch collapse.
        let mut queued = HashSet::new();
        let mut queue = Vec::new();
        for root in roots {
            let project = match self.content.project(&root.source, &root.project).await {
                Ok(project) => project,
                Err(e) => {
                    failures.push(failure(&root.given, "", format!("{e:#}")));
                    continue;
                }
            };
            if project.kind != kind {
                failures.push(failure(
                    &root.given,
                    &project.title,
                    format!(
                        "'{}' is {:?} content, not {:?}",
                        project.title, project.kind, kind
                    ),
                ));
                continue;
            }
            if let Err(e) = side_gate(&project, ctx.side) {
                failures.push(failure(&root.given, &project.title, format!("{e:#}")));
                continue;
            }
            if !queued.insert(project.id.clone()) {
                continue;
            }
            visited.insert(project.id.clone());
            queue.push(Node {
                given: root.given,
                source: root.source,
                pin: root.pin,
                project,
            });
        }

        while let Some(node) = queue.pop() {
            let versions = match self
                .content
                .versions(&VersionQuery {
                    source: node.source.clone(),
                    project: node.project.id.clone(),
                    loader: loader.clone(),
                    game_version: Some(ctx.game_version.clone()),
                })
                .await
            {
                Ok(versions) => versions,
                Err(e) => {
                    failures.push(failure(&node.given, &node.project.title, format!("{e:#}")));
                    continue;
                }
            };
            let version = match install::pick_version(
                &versions,
                &ctx.game_version,
                loader.as_deref(),
                &node.pin,
            ) {
                Ok(version) => version.clone(),
                Err(e) => {
                    failures.push(failure(
                        &node.given,
                        &node.project.title,
                        format!("cannot install '{}': {e:#}", node.project.title),
                    ));
                    continue;
                }
            };

            if kind == ContentKind::Mod {
                for dep in &version.dependencies {
                    if dep.kind != DependencyKind::Required {
                        continue;
                    }
                    if dep.project_id.is_empty() {
                        tracing::warn!(
                            of = %node.project.title,
                            version_id = %dep.version_id,
                            "required dependency names no project; skipping"
                        );
                        continue;
                    }
                    if !visited.insert(dep.project_id.clone()) {
                        continue;
                    }
                    let dep_project =
                        match self.content.project(&node.source, &dep.project_id).await {
                            Ok(project) => project,
                            Err(e) => {
                                failures.push(failure(&dep.project_id, "", format!("{e:#}")));
                                continue;
                            }
                        };
                    if side_gate(&dep_project, ctx.side).is_err() {
                        tracing::warn!(
                            dependency = %dep_project.title,
                            of = %node.project.title,
                            "required dependency does not support this side; skipping"
                        );
                        continue;
                    }
                    queue.push(Node {
                        given: dep_project.slug.clone(),
                        source: node.source.clone(),
                        pin: String::new(),
                        project: dep_project,
                    });
                }
            }

            let title = node.project.title.clone();
            let labeled = move |p: &ProvisionProgress| {
                let mut progress = p.clone();
                progress.detail = title.clone();
                on_progress(&progress);
            };
            match self
                .install_version_file(ctx, &node.project, &version, worlds, &labeled)
                .await
            {
                Ok(mut installed) => items.append(&mut installed),
                Err(e) => {
                    failures.push(failure(&node.given, &node.project.title, format!("{e:#}")))
                }
            }
        }
        (items, failures)
    }

    /// Download a version's primary file into every target world (one entry
    /// per world; non-datapack kinds pass the single empty world).
    async fn install_version_file(
        &self,
        ctx: &EntryContent,
        project: &ContentProject,
        version: &proto::content::ContentVersion,
        worlds: &[String],
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        let file = install::primary_file(version)?;
        materialize::validate_filename(&file.artifact.filename)?;
        let mut installed = Vec::new();
        for world in worlds {
            let (managed, data) =
                content_targets(ctx, project.kind, world, &file.artifact.filename)?;
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
            installed.push(InstalledContent {
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
            });
        }
        Ok(installed)
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
                .install_version_file(
                    ctx,
                    &project,
                    &version,
                    std::slice::from_ref(&item.world),
                    on_progress,
                )
                .await?
                .into_iter()
                .next()
                .context("install produced no item")?;
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

/// One platform selector of a batch, resolved from its item: where it came
/// from (`given`, for failure reporting), which source serves it, and the
/// version pin.
struct PlatformRoot {
    given: String,
    source: String,
    project: String,
    pin: String,
}

/// A BFS node: a fetched project awaiting version resolution and install.
struct Node {
    given: String,
    source: String,
    pin: String,
    project: ContentProject,
}

fn failure(
    item: impl Into<String>,
    title: impl Into<String>,
    message: impl Into<String>,
) -> ContentFailure {
    ContentFailure {
        item: item.into(),
        title: title.into(),
        message: message.into(),
    }
}

/// The selector an item names, for failure reporting on malformed items.
fn item_label(item: &ContentAddItem) -> String {
    [&item.project, &item.url, &item.path]
        .iter()
        .find(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "(empty item)".to_string())
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

/// Resolve the data-relative world directories a datapack batch installs into
/// — a server's single `level-name` world, or an instance's chosen (and
/// validated) saves. The single empty world for every non-datapack kind,
/// which installs into a flat dir.
fn datapack_worlds(ctx: &EntryContent, spec: &ContentAddSpec) -> Result<Vec<String>> {
    if spec.kind != ContentKind::DataPack {
        return Ok(vec![String::new()]);
    }
    match ctx.side {
        EntrySide::Server => Ok(vec![crate::servers::level_name(&ctx.data_dir)]),
        EntrySide::Client => {
            let mut worlds = Vec::new();
            for world in &spec.worlds {
                let requested = world.trim();
                if requested.is_empty() {
                    continue;
                }
                if !ctx.data_dir.join("saves").join(requested).is_dir() {
                    bail!("no save world '{requested}' in this instance");
                }
                let resolved = format!("saves/{requested}");
                if !worlds.contains(&resolved) {
                    worlds.push(resolved);
                }
            }
            if worlds.is_empty() {
                bail!("name a save world for the datapack (the instance's --world)");
            }
            Ok(worlds)
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

/// Import a local file: copy it into the managed directory (or, for a
/// datapack, straight into each target world) and mirror it into the game
/// directory. No provenance beyond the hash, so it can never update.
fn add_file_content(
    ctx: &EntryContent,
    kind: ContentKind,
    item: &ContentAddItem,
    worlds: &[String],
) -> Result<Vec<InstalledContent>> {
    let source = Path::new(&item.path);
    if !source.is_file() {
        bail!("no file at {}", source.display());
    }
    let filename = if item.filename.is_empty() {
        source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        item.filename.clone()
    };
    materialize::validate_filename(&filename)?;
    let mut installed = Vec::new();
    for world in worlds {
        let (managed, data) = content_targets(ctx, kind, world, &filename)?;
        if let Some(parent) = managed.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("cannot create {}", parent.display()))?;
        }
        std::fs::copy(source, &managed)
            .with_context(|| format!("cannot import {}", source.display()))?;
        if managed != data {
            install::mirror(&managed, &data)?;
        }
        installed.push(InstalledContent {
            kind,
            source: "file".to_string(),
            title: filename.clone(),
            sha1: install::sha1_file(&managed)?,
            filename: filename.clone(),
            installed_unix: registry::now_unix(),
            world: world.to_string(),
            ..InstalledContent::default()
        });
    }
    Ok(installed)
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
