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
use proto::error::ErrorInfo;
use proto::minecraft::{ProvisionPhase, ProvisionProgress};

use super::phase_progress;
use crate::content::{inspect, install, profiles};
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
        let (_, ctx) = self.server_content_ctx(reference)?;
        self.ensure_accepts(&ctx, spec.kind)?;
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
        let (_, ctx) = self.instance_content_ctx(reference)?;
        self.ensure_accepts(&ctx, spec.kind)?;
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

    /// Uninstall one item (matched by project id, slug, filename, or title) —
    /// its managed copy and every mirror of it. A non-empty `worlds` instead
    /// narrows a datapack to the worlds it keeps loading in. False when nothing
    /// matches.
    pub fn remove_server_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        worlds: &[String],
    ) -> Result<bool> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        Ok(!remove_content(&ctx, kind, item, worlds)?.is_empty())
    }

    pub fn remove_instance_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        worlds: &[String],
    ) -> Result<bool> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        // A global-profile install is owned by its profile: removing it locally
        // would silently reappear on the next apply, so the removal is refused
        // until the reference leaves the profile (no local-exclusion mechanism).
        let tagged = install::load(&ctx.entry_dir).into_iter().find(|i| {
            i.kind == kind && install::matches(i, item) && i.origin.starts_with("profile:")
        });
        if let Some(tagged) = tagged {
            bail!(
                "'{}' was installed by global profile '{}'; remove it from that profile instead",
                tagged.title,
                tagged.origin.trim_start_matches("profile:")
            );
        }
        let removed = remove_content(&ctx, kind, item, worlds)?;
        let gone: Vec<String> = removed
            .iter()
            .filter(|i| profiles::selectable(i.kind))
            .map(|i| i.filename.clone())
            .collect();
        profiles::prune(&ctx.entry_dir, &gone)?;
        Ok(!removed.is_empty())
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
        self.update_content(&ctx, kind, item, "", on_progress).await
    }

    pub async fn update_instance_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        self.change_instance_version(reference, kind, item, "", on_progress)
            .await
    }

    /// Re-pin one named platform item to a specific published `version` (id or
    /// number), re-installing that version like an update.
    pub async fn set_server_content_version(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        version: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        if item.is_empty() || version.is_empty() {
            bail!(proto::error::ErrorInfo::FieldsRequired {
                fields: vec![proto::error::Field::Item, proto::error::Field::Version]
            });
        }
        let (_, ctx) = self.server_content_ctx(reference)?;
        self.update_content(&ctx, kind, item, version, on_progress)
            .await
    }

    pub async fn set_instance_content_version(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        version: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        if item.is_empty() || version.is_empty() {
            bail!(proto::error::ErrorInfo::FieldsRequired {
                fields: vec![proto::error::Field::Item, proto::error::Field::Version]
            });
        }
        self.change_instance_version(reference, kind, item, version, on_progress)
            .await
    }

    /// The instance version-change path shared by update (empty pin) and
    /// set-version (explicit pin): apply the change, then follow each item's
    /// filename move in every content profile.
    async fn change_instance_version(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        pin: &str,
        on_progress: OnProgress<'_>,
    ) -> Result<Vec<InstalledContent>> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        let before = install::load(&ctx.entry_dir);
        let updated = self
            .update_content(&ctx, kind, item, pin, on_progress)
            .await?;
        for new_item in &updated {
            let old = before
                .iter()
                .find(|i| i.kind == new_item.kind && i.project_id == new_item.project_id);
            if let Some(old) = old {
                profiles::remap(&ctx.entry_dir, &old.filename, &new_item.filename)?;
            }
        }
        Ok(updated)
    }

    /// Enable or disable installed items matching `item`; a non-empty `worlds`
    /// scopes a datapack toggle to those save worlds. Returns the number of
    /// matched entries — zero means nothing matched. The entry must be stopped
    /// (enforced at the service boundary).
    pub fn enable_server_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        enabled: bool,
        worlds: &[String],
    ) -> Result<usize> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        set_enabled(&ctx, kind, item, enabled, worlds)
    }

    pub fn enable_instance_content(
        &self,
        reference: &str,
        kind: ContentKind,
        item: &str,
        enabled: bool,
        worlds: &[String],
    ) -> Result<usize> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        set_enabled(&ctx, kind, item, enabled, worlds)
    }

    /// For each platform-sourced item of the kind, resolve the newest
    /// compatible version and report whether it differs from the current pin.
    /// An item whose versions cannot be resolved is skipped, not fatal.
    pub async fn check_server_updates(
        &self,
        reference: &str,
        kind: ContentKind,
    ) -> Result<Vec<proto::content::ContentUpdate>> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        self.content_updates(&ctx, kind).await
    }

    pub async fn check_instance_updates(
        &self,
        reference: &str,
        kind: ContentKind,
    ) -> Result<Vec<proto::content::ContentUpdate>> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        self.content_updates(&ctx, kind).await
    }

    async fn content_updates(
        &self,
        ctx: &EntryContent,
        kind: ContentKind,
    ) -> Result<Vec<proto::content::ContentUpdate>> {
        let items: Vec<InstalledContent> = install::load(&ctx.entry_dir)
            .into_iter()
            .filter(|i| i.kind == kind && !i.project_id.is_empty())
            .collect();
        let loader = content_loader(kind, &ctx.flavor);
        let mut out = Vec::new();
        for item in items {
            let versions = match self
                .content
                .versions(&VersionQuery {
                    source: item.source.clone(),
                    project: item.project_id.clone(),
                    loader: loader.clone(),
                    game_version: Some(ctx.game_version.clone()),
                })
                .await
            {
                Ok(versions) => versions,
                Err(e) => {
                    tracing::warn!(title = %item.title, error = %format!("{e:#}"), "update check failed");
                    continue;
                }
            };
            let Ok(latest) =
                install::pick_version(&versions, &ctx.game_version, loader.as_deref(), "")
            else {
                continue;
            };
            out.push(proto::content::ContentUpdate {
                filename: item.filename,
                project_id: item.project_id,
                current_version_id: item.version_id.clone(),
                current_version_number: item.version_number.clone(),
                latest_version_id: latest.id.clone(),
                latest_version_number: latest.version_number.clone(),
                updatable: latest.id != item.version_id,
            });
        }
        Ok(out)
    }

    fn server_content_ctx(&self, reference: &str) -> Result<(ServerRecord, EntryContent)> {
        let record = self
            .servers
            .get(reference)
            .with_context(|| format!("unknown server: {reference}"))?;
        if !record.ready() {
            bail!(proto::error::ErrorInfo::Provisioning {
                name: record.name.clone()
            });
        }
        let ctx = EntryContent {
            entry_dir: self.servers.server_dir(&record),
            data_dir: self.servers.data_dir(&record),
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
            entry_dir: self.instances.instance_dir(&record),
            data_dir: self.instances.data_dir(&record),
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
        if spec.items.is_empty() {
            bail!(proto::error::ErrorInfo::NothingToDo {
                what: proto::error::Task::Install
            });
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
                    ErrorInfo::MutuallyExclusive {
                        options: vec!["a project".into(), "a url".into(), "a file".into()],
                    },
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
                        ErrorInfo::UnsupportedContentUrl {
                            url: item.url.clone(),
                        },
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
                Ok(installed) => items.push(installed),
                Err(e) => failures.push(failure(&item.path, "", crate::error_info(e))),
            }
        }
        let (mut platform_items, mut platform_failures) = self
            .add_platform_content(ctx, spec.kind, roots, &worlds, on_progress)
            .await;
        items.append(&mut platform_items);
        failures.append(&mut platform_failures);

        let entry_worlds = ctx.worlds();
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
                    install::remove_files(&ctx.entry_dir, &ctx.data_dir, &old, &entry_worlds);
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
                error = %fail.error,
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
        on_progress.report(&phase_progress(ProvisionPhase::Resolving));

        let mut visited: HashSet<String> = install::load(&ctx.entry_dir)
            .into_iter()
            .map(|i| i.project_id)
            .filter(|p| !p.is_empty())
            .collect();
        let loader = content_loader(kind, &ctx.flavor);

        // An explicitly named root installs even when already present (a
        // reinstall/re-pin); only duplicates within the batch collapse.
        let mut queued = HashSet::new();
        let mut queue = Vec::new();
        for root in roots {
            let project = match self
                .content
                .project(&root.source, &root.project, Some(kind))
                .await
            {
                Ok(project) => project,
                Err(e) => {
                    failures.push(failure(&root.given, "", crate::error_info(e)));
                    continue;
                }
            };
            if !kind_matches(kind, project.kind) {
                failures.push(failure(
                    &root.given,
                    &project.title,
                    ErrorInfo::ContentKindMismatch {
                        title: project.title.clone(),
                        actual: project.kind,
                        expected: kind,
                    },
                ));
                continue;
            }
            if let Err(e) = side_gate(kind, &project, ctx.side) {
                failures.push(failure(&root.given, &project.title, crate::error_info(e)));
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

        let mut finished = 0u64;
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
                    failures.push(failure(
                        &node.given,
                        &node.project.title,
                        crate::error_info(e),
                    ));
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
                        crate::error_info(e),
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
                    let dep_project = match self
                        .content
                        .project(&node.source, &dep.project_id, Some(kind))
                        .await
                    {
                        Ok(project) => project,
                        Err(e) => {
                            failures.push(failure(&dep.project_id, "", crate::error_info(e)));
                            continue;
                        }
                    };
                    if side_gate(kind, &dep_project, ctx.side).is_err() {
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

            // Label each forwarded event with which unit of the batch this
            // is: the queue length is live, so `items` grows as dependency
            // resolution discovers more work.
            let title = node.project.title.clone();
            let item = finished + 1;
            let known = item + queue.len() as u64;
            let relabel = move |p: &ProvisionProgress| {
                let mut progress = p.clone();
                progress.detail = title.clone();
                progress.item = item;
                progress.items = known;
                on_progress.report(&progress);
            };
            // The relabelled view keeps the same cancel token: per-item install
            // is the longest part of a batch, and must stay interruptible.
            let labeled = crate::cancel::Job::new(&relabel, on_progress.cancel());
            labeled.report(&phase_progress(ProvisionPhase::Content));
            match self
                .install_version_file(ctx, kind, &node.project, &version, worlds, &labeled)
                .await
            {
                Ok(installed) => items.push(installed),
                Err(e) => failures.push(failure(
                    &node.given,
                    &node.project.title,
                    crate::error_info(e),
                )),
            }
            finished += 1;
        }
        (items, failures)
    }

    /// Download a version's primary file into the managed directory and mirror
    /// it into the game's load dirs. `kind` is the *requested* kind, not the
    /// project's: Modrinth types datapacks as mod projects, so `project.kind`
    /// would route a datapack into `mods/`.
    async fn install_version_file(
        &self,
        ctx: &EntryContent,
        kind: ContentKind,
        project: &ContentProject,
        version: &proto::content::ContentVersion,
        worlds: &[String],
        on_progress: OnProgress<'_>,
    ) -> Result<InstalledContent> {
        let file = install::primary_file(version)?;
        materialize::validate_filename(&file.artifact.filename)?;
        let managed = content_target(ctx, kind, &file.artifact.filename)?;
        materialize::ensure_artifact(
            Some(&self.cache),
            &file.artifact,
            &managed,
            ProvisionPhase::Content,
            on_progress,
        )
        .await?;
        let item = InstalledContent {
            kind,
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
            icon_url: project.icon_url.clone(),
            installed_unix: registry::now_unix(),
            worlds: worlds.to_vec(),
            origin: String::new(),
            enabled: true,
            disabled_worlds: Vec::new(),
        };
        install::apply_files(&ctx.entry_dir, &ctx.data_dir, &item, &ctx.worlds())?;
        Ok(item)
    }

    /// Move matched platform items to a newer version — the newest compatible
    /// when `pin` is empty, or that exact version (id or number) when pinned.
    async fn update_content(
        &self,
        ctx: &EntryContent,
        kind: ContentKind,
        reference: &str,
        pin: &str,
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
                false => bail!(proto::error::ErrorInfo::ContentNotFound {
                    reference: reference.to_string()
                }),
            }
        }
        let loader = content_loader(kind, &ctx.flavor);

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
            on_progress.report(&phase_progress(ProvisionPhase::Resolving));
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
                install::pick_version(&versions, &ctx.game_version, loader.as_deref(), pin)
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
                    item.kind,
                    &project,
                    &version,
                    &item.worlds,
                    on_progress,
                )
                .await?;
            if new_item.filename != item.filename {
                install::remove_files(&ctx.entry_dir, &ctx.data_dir, &item, &ctx.worlds());
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
                    Some(entry) => {
                        // An update moves the version, not the ownership or the
                        // enabled state: a profile-tagged, disabled item keeps
                        // both.
                        let origin = std::mem::take(&mut entry.origin);
                        let enabled = entry.enabled;
                        *entry = new_item.clone();
                        entry.origin = origin;
                        entry.enabled = enabled;
                    }
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

/// The loader filter a kind's version lookup needs: the entry's own loader for
/// whatever its flavor loads (a mod on fabric, a plugin on paper — Modrinth
/// names both by the flavor), and the `datapack` pseudo-loader for datapacks —
/// Modrinth types datapacks as mods carrying that loader, so the filter is what
/// selects the datapack file over a jar. Folia is filtered as `folia`, not
/// widened to `paper`: a plugin that never claimed Folia support deadlocks on
/// its regionised scheduler.
fn content_loader(kind: ContentKind, flavor: &str) -> Option<String> {
    match kind {
        ContentKind::Mod | ContentKind::Plugin => Some(flavor.to_string()),
        ContentKind::DataPack => Some("datapack".to_string()),
        _ => None,
    }
}

/// Whether a fetched project satisfies the requested kind. Datapacks accept
/// `Mod` projects: Modrinth has no datapack project type — a datapack is a
/// mod-typed project whose versions carry the `datapack` loader.
fn kind_matches(requested: ContentKind, project: ContentKind) -> bool {
    requested == project || (requested == ContentKind::DataPack && project == ContentKind::Mod)
}

fn failure(item: impl Into<String>, title: impl Into<String>, error: ErrorInfo) -> ContentFailure {
    ContentFailure {
        item: item.into(),
        title: title.into(),
        error,
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

impl EntryContent {
    /// The entry's worlds, data-relative: a server's single `level-name`, an
    /// instance's save folders.
    fn worlds(&self) -> Vec<String> {
        match self.side {
            EntrySide::Server => vec![crate::servers::level_name(&self.data_dir)],
            EntrySide::Client => crate::instances::save_worlds(&self.data_dir),
        }
    }
}

impl Engine {
    /// Refuse a kind the entry cannot load, naming what it can. Composed from
    /// two independent facts: what the *flavor's* loader consumes (mods on
    /// fabric, plugins on paper, nothing on vanilla) and what the *side* reads
    /// for itself — a client its resourcepacks and shaders, either side the
    /// datapacks that are world data rather than loader content.
    fn ensure_accepts(&self, ctx: &EntryContent, requested: ContentKind) -> Result<()> {
        let accepts = self.accepted_kinds(ctx);
        if accepts.contains(&requested) {
            return Ok(());
        }
        bail!(proto::error::ErrorInfo::ContentKindRejected {
            entry: match ctx.side {
                EntrySide::Server => proto::error::EntryKind::Server,
                EntrySide::Client => proto::error::EntryKind::Instance,
            },
            flavor: ctx.flavor.clone(),
            requested,
            accepts,
        })
    }

    fn accepted_kinds(&self, ctx: &EntryContent) -> Vec<ContentKind> {
        let loads = match ctx.side {
            EntrySide::Server => self.minecraft().server_loads(&ctx.flavor),
            EntrySide::Client => self.minecraft().instance_loads(&ctx.flavor),
        };
        accepted_kinds(ctx.side, loads)
    }
}

/// The two facts composed: whatever the flavor's loader takes, plus what the
/// side reads for itself — a client its own resourcepacks and shaders, either
/// side the datapacks that are world data rather than loader content.
fn accepted_kinds(side: EntrySide, loads: Option<ContentKind>) -> Vec<ContentKind> {
    let mut kinds: Vec<ContentKind> = loads.into_iter().collect();
    if side == EntrySide::Client {
        kinds.push(ContentKind::ResourcePack);
        kinds.push(ContentKind::Shader);
    }
    kinds.push(ContentKind::DataPack);
    kinds
}

/// Reject content the platform marks unsupported for the entry's side
/// (`Unknown` passes — the platform did not say). Datapacks are exempt: they
/// run on the server side of any world, including a client's integrated server,
/// so a source's client-side flag must not block installing one on an instance.
/// Judged by the *requested* kind — Modrinth types datapacks as mod projects,
/// so `project.kind` would miss the exemption.
fn side_gate(requested: ContentKind, project: &ContentProject, side: EntrySide) -> Result<()> {
    if requested == ContentKind::DataPack {
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

/// The save worlds a datapack batch targets, data-relative: an instance's
/// chosen (and validated) saves, empty meaning every world it has now or grows
/// later. A server has one world, so its selection is always empty; so is every
/// non-datapack kind, which mirrors into a flat dir.
fn datapack_worlds(ctx: &EntryContent, spec: &ContentAddSpec) -> Result<Vec<String>> {
    if spec.kind != ContentKind::DataPack || ctx.side == EntrySide::Server {
        return Ok(Vec::new());
    }
    let mut worlds = Vec::new();
    for world in &spec.worlds {
        let requested = world.trim();
        if requested.is_empty() {
            continue;
        }
        if !ctx.data_dir.join("saves").join(requested).is_dir() {
            bail!(proto::error::ErrorInfo::WorldNotFound {
                world: requested.to_string()
            });
        }
        let resolved = format!("saves/{requested}");
        if !worlds.contains(&resolved) {
            worlds.push(resolved);
        }
    }
    Ok(worlds)
}

/// The managed path a kind's file occupies under the entry root — the source of
/// truth every mirror is placed from.
fn content_target(ctx: &EntryContent, kind: ContentKind, filename: &str) -> Result<PathBuf> {
    Ok(ctx.entry_dir.join(install::kind_dir(kind)?).join(filename))
}

/// Import a local file: copy it into the managed directory and mirror it into
/// the game's load dirs. No provenance beyond the hash, so it can never update.
fn add_file_content(
    ctx: &EntryContent,
    kind: ContentKind,
    item: &ContentAddItem,
    worlds: &[String],
) -> Result<InstalledContent> {
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
    // Reject an unreadable archive or a modpack; the detected kind is advisory
    // (the requested `kind` wins, so an override installs where asked).
    match inspect::classify(source)? {
        inspect::Detected::Kind(detected) if detected != kind => tracing::warn!(
            file = %filename,
            ?detected,
            requested = ?kind,
            "importing a local file under a kind that differs from its detected type"
        ),
        _ => {}
    }
    let managed = content_target(ctx, kind, &filename)?;
    if let Some(parent) = managed.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    std::fs::copy(source, &managed)
        .with_context(|| format!("cannot import {}", source.display()))?;
    let installed = InstalledContent {
        kind,
        source: "file".to_string(),
        title: filename.clone(),
        sha1: install::sha1_file(&managed)?,
        filename,
        installed_unix: registry::now_unix(),
        worlds: worlds.to_vec(),
        enabled: true,
        ..InstalledContent::default()
    };
    install::apply_files(&ctx.entry_dir, &ctx.data_dir, &installed, &ctx.worlds())?;
    Ok(installed)
}

fn list_content(ctx: &EntryContent, kind: ContentKind) -> (Vec<InstalledContent>, Vec<String>) {
    let items: Vec<InstalledContent> = install::load(&ctx.entry_dir)
        .into_iter()
        .filter(|i| i.kind == kind)
        .collect();
    let untracked = install::untracked(&ctx.data_dir, kind, &items, &ctx.worlds());
    (items, untracked)
}

/// Uninstall matching items: the managed copy and every mirror of it. A
/// non-empty `worlds` (datapacks only) instead drops those worlds from the
/// item's selection, uninstalling it only when none is left.
fn remove_content(
    ctx: &EntryContent,
    kind: ContentKind,
    reference: &str,
    worlds: &[String],
) -> Result<Vec<InstalledContent>> {
    if !worlds.is_empty() && kind != ContentKind::DataPack {
        bail!(proto::error::ErrorInfo::UnsupportedOperation {
            reason: proto::error::Unsupported::DatapacksPerWorld
        });
    }
    let entry_worlds = ctx.worlds();
    let (matched, mut kept): (Vec<_>, Vec<_>) = install::load(&ctx.entry_dir)
        .into_iter()
        .partition(|i| i.kind == kind && install::matches(i, reference));
    if matched.is_empty() {
        return Ok(matched);
    }
    let mut removed = Vec::new();
    for mut item in matched {
        let remaining = keep_worlds(&item, worlds, &entry_worlds);
        if !remaining.is_empty() {
            item.worlds = remaining;
            install::apply_files(&ctx.entry_dir, &ctx.data_dir, &item, &entry_worlds)?;
            tracing::info!(
                entry = %ctx.entry_dir.display(),
                title = %item.title,
                worlds = ?item.worlds,
                "datapack narrowed to fewer worlds"
            );
            kept.push(item);
            continue;
        }
        install::remove_files(&ctx.entry_dir, &ctx.data_dir, &item, &entry_worlds);
        tracing::info!(
            entry = %ctx.entry_dir.display(),
            kind = ?item.kind,
            title = %item.title,
            filename = %item.filename,
            "content removed"
        );
        removed.push(item);
    }
    install::save(&ctx.entry_dir, kept)?;
    Ok(removed)
}

/// The worlds an item still loads in once `drop` are taken from it; empty means
/// nothing is left, so the item goes. An "every world" selection materialises to
/// the entry's current worlds first.
fn keep_worlds(item: &InstalledContent, drop: &[String], entry_worlds: &[String]) -> Vec<String> {
    if drop.is_empty() {
        return Vec::new();
    }
    let current = match item.worlds.is_empty() {
        true => entry_worlds,
        false => &item.worlds,
    };
    current
        .iter()
        .filter(|world| !drop.iter().any(|named| install::world_name(world) == named))
        .cloned()
        .collect()
}

/// Toggle every index entry matching `reference`, applying the filesystem side
/// immediately. With no `worlds` the item itself flips; with worlds (datapacks
/// only) just those are scoped in or out, leaving the rest as they are. Returns
/// how many entries matched (regardless of whether anything moved), so the
/// caller can distinguish "nothing matched" from "already in that state".
fn set_enabled(
    ctx: &EntryContent,
    kind: ContentKind,
    reference: &str,
    enabled: bool,
    scope: &[String],
) -> Result<usize> {
    if !scope.is_empty() && kind != ContentKind::DataPack {
        bail!(proto::error::ErrorInfo::UnsupportedOperation {
            reason: proto::error::Unsupported::DatapacksPerWorld
        });
    }
    let entry_worlds = ctx.worlds();
    let mut index = install::load(&ctx.entry_dir);
    let mut matched = 0usize;
    for item in index.iter_mut() {
        if item.kind != kind || !install::matches(item, reference) {
            continue;
        }
        matched += 1;
        scope_enabled(item, enabled, scope, &entry_worlds);
        install::apply_files(&ctx.entry_dir, &ctx.data_dir, item, &entry_worlds)?;
        tracing::info!(
            entry = %ctx.entry_dir.display(),
            title = %item.title,
            filename = %item.filename,
            enabled,
            worlds = ?scope,
            "content enabled state changed"
        );
    }
    if matched > 0 {
        install::save(&ctx.entry_dir, index)?;
    }
    Ok(matched)
}

/// Apply a toggle to one item: item-wide with no `scope`, else only to the
/// named worlds. Enabling a world also turns the item itself back on, since a
/// wholly disabled pack loads nowhere.
fn scope_enabled(
    item: &mut InstalledContent,
    enabled: bool,
    scope: &[String],
    entry_worlds: &[String],
) {
    if scope.is_empty() {
        item.enabled = enabled;
        if enabled {
            item.disabled_worlds.clear();
        }
        return;
    }
    let targets: Vec<String> = install::target_worlds(item, entry_worlds)
        .iter()
        .filter(|world| {
            scope
                .iter()
                .any(|named| install::world_name(world) == named)
        })
        .cloned()
        .collect();
    if enabled {
        item.enabled = true;
        item.disabled_worlds
            .retain(|world| !targets.contains(world));
    } else {
        for world in targets {
            if !item.disabled_worlds.contains(&world) {
                item.disabled_worlds.push(world);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EntryContent {
        EntryContent {
            entry_dir: PathBuf::from("/entry"),
            data_dir: PathBuf::from("/entry/data"),
            game_version: "1.21.1".to_string(),
            flavor: "fabric".to_string(),
            side: EntrySide::Client,
        }
    }

    #[test]
    fn a_flavor_contributes_its_loader_kind_and_the_side_the_rest() {
        assert_eq!(
            accepted_kinds(EntrySide::Server, Some(ContentKind::Plugin)),
            vec![ContentKind::Plugin, ContentKind::DataPack],
            "a paper server takes plugins, never mods"
        );
        assert_eq!(
            accepted_kinds(EntrySide::Server, Some(ContentKind::Mod)),
            vec![ContentKind::Mod, ContentKind::DataPack]
        );
        assert_eq!(
            accepted_kinds(EntrySide::Server, None),
            vec![ContentKind::DataPack],
            "vanilla loads nothing of its own, but a world still takes datapacks"
        );
        assert_eq!(
            accepted_kinds(EntrySide::Client, None),
            vec![
                ContentKind::ResourcePack,
                ContentKind::Shader,
                ContentKind::DataPack
            ],
            "a client reads packs whatever its flavor loads"
        );
    }

    #[test]
    fn targets_route_by_requested_kind_not_project_kind() {
        assert_eq!(
            content_target(&ctx(), ContentKind::DataPack, "pack.zip").unwrap(),
            Path::new("/entry/datapacks/pack.zip")
        );
        assert_eq!(
            content_target(&ctx(), ContentKind::Shader, "shader.zip").unwrap(),
            Path::new("/entry/shaderpacks/shader.zip")
        );
    }

    #[test]
    fn scoping_a_world_off_leaves_the_rest_loaded() {
        let entry_worlds = vec!["saves/one".to_string(), "saves/two".to_string()];
        let mut item = InstalledContent {
            kind: ContentKind::DataPack,
            enabled: true,
            ..InstalledContent::default()
        };

        scope_enabled(&mut item, false, &["two".to_string()], &entry_worlds);
        assert_eq!(item.disabled_worlds, vec!["saves/two".to_string()]);
        assert!(item.enabled, "the item itself stays on");

        scope_enabled(&mut item, true, &["two".to_string()], &entry_worlds);
        assert!(item.disabled_worlds.is_empty());
    }

    #[test]
    fn removing_named_worlds_keeps_the_rest() {
        let entry_worlds = vec!["saves/one".to_string(), "saves/two".to_string()];
        let item = InstalledContent {
            kind: ContentKind::DataPack,
            ..InstalledContent::default()
        };
        assert_eq!(
            keep_worlds(&item, &["two".to_string()], &entry_worlds),
            vec!["saves/one".to_string()]
        );
        assert!(
            keep_worlds(
                &item,
                &["one".to_string(), "two".to_string()],
                &entry_worlds
            )
            .is_empty(),
            "no world left means the item goes"
        );
        assert!(
            keep_worlds(&item, &[], &entry_worlds).is_empty(),
            "an unscoped removal takes the item"
        );
    }

    #[test]
    fn side_gate_waives_datapacks_by_requested_kind() {
        // Modrinth types datapacks as mod projects, often client-unsupported.
        let project = ContentProject {
            kind: ContentKind::Mod,
            client_side: SideSupport::Unsupported,
            ..ContentProject::default()
        };
        assert!(side_gate(ContentKind::DataPack, &project, EntrySide::Client).is_ok());
        assert!(side_gate(ContentKind::Mod, &project, EntrySide::Client).is_err());
    }
}
