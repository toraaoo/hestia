//! Putting a resolved pack onto an entry: which of its files go where, what is
//! downloaded, what is written into the game directory, and how the result
//! reconciles against whatever pack was installed before.

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use proto::content::{
    ContentFailure, ContentKind, ContentProject, InstalledContent, ModpackFile, ResolvedModpack,
};
use proto::error::ErrorInfo;
use proto::minecraft::{ProvisionPhase, ProvisionProgress};
use proto::modpack::{InstalledModpack, ModpackOverride};
use proto::warning::WarningInfo;

use super::super::content::entry::{content_target, EntryContent, EntrySide};
use super::super::phase_progress;
use super::{pack_display_name, FileIdentity};
use crate::cancel::Job;
use crate::content::{install, modpack, pack};
use crate::engine::Engine;
use crate::minecraft::materialize;
use crate::registry;

impl Engine {
    /// The install itself, once the entry exists: refuse a pack the entry
    /// cannot run, download its managed files, write its game-directory files,
    /// and reconcile against whatever pack was there before.
    pub(super) async fn apply_pack(
        &self,
        ctx: &EntryContent,
        resolved: &ResolvedModpack,
        project: Option<&ContentProject>,
        archive: &mut pack::Archive,
        job: &Job<'_>,
    ) -> Result<(InstalledModpack, Vec<ContentFailure>, Vec<WarningInfo>)> {
        self.ensure_entry_matches(ctx, resolved)?;
        let previous = modpack::load(&ctx.entry_dir);
        let side = match ctx.side {
            EntrySide::Server => pack::Side::Server,
            EntrySide::Client => pack::Side::Client,
        };
        let accepts = self.accepted_kinds(ctx);
        let mut warnings = Vec::new();

        let wanted: Vec<&ModpackFile> = resolved.files.iter().filter(|f| side.wants(f)).collect();
        let mut managed = Vec::new();
        let mut loose = Vec::new();
        let mut rejected = 0u32;
        for file in wanted {
            match managed_kind(&file.path) {
                Some(kind) if accepts.contains(&kind) => managed.push((kind, file)),
                // A kind the flavor cannot load is not a failure — a
                // client-shaped pack on a server ships plenty of them — but it
                // is worth saying once, since the pack will not play as built.
                Some(_) => rejected += 1,
                None => loose.push(file),
            }
        }
        if rejected > 0 {
            warnings.push(WarningInfo::ModpackFilesNotAccepted {
                count: rejected,
                flavor: ctx.flavor.clone(),
            });
        }

        let (items, failures) = self.fetch_pack_content(ctx, &managed, resolved, job).await;
        let mut overrides = self
            .fetch_loose_files(ctx, &loose, previous.as_ref(), job)
            .await;
        let mut kept: Vec<String> = Vec::new();

        job.check()?;
        let owned: HashMap<&str, &ModpackOverride> = previous
            .as_ref()
            .map(|p| p.overrides.iter().map(|o| (o.path.as_str(), o)).collect())
            .unwrap_or_default();
        let data_dir = ctx.data_dir.clone();
        let written = archive.extract_overrides(side, &ctx.data_dir, job, |path| {
            let writable = match owned.get(path) {
                // Known to the previous pack: only overwrite what is still
                // byte-for-byte what we wrote.
                Some(previous) => modpack::ours(&data_dir, previous),
                // Unknown: an existing file was not put there by a pack, so it
                // is the user's (or the game's) and is not ours to replace.
                None => !data_dir.join(path).exists(),
            };
            if !writable {
                kept.push(path.to_string());
            }
            writable
        })?;
        overrides.extend(written.into_iter().map(|w| ModpackOverride {
            path: w.path,
            sha1: w.sha1,
        }));

        let pack = InstalledModpack {
            source: resolved.source.clone(),
            project_id: resolved.project_id.clone(),
            slug: project.map(|p| p.slug.clone()).unwrap_or_default(),
            name: pack_display_name(resolved, project),
            version_id: resolved.version_id.clone(),
            version_number: resolved.version_number.clone(),
            game_version: resolved.game_version.clone(),
            loader: resolved.loader.clone().unwrap_or_default(),
            loader_version: resolved.loader_version.clone().unwrap_or_default(),
            icon_url: project.map(|p| p.icon_url.clone()).unwrap_or_default(),
            installed_unix: registry::now_unix(),
            files: items.iter().map(|i| i.filename.clone()).collect(),
            overrides,
        };

        self.merge_pack_items(ctx, &pack, items, previous.as_ref())?;
        if let Some(previous) = &previous {
            let stale = stale_overrides(previous, &pack);
            let (_, mut left) = modpack::remove_overrides(&ctx.data_dir, &stale);
            kept.append(&mut left);
        }
        if !kept.is_empty() {
            kept.sort();
            kept.dedup();
            warnings.push(WarningInfo::ModpackOverridesKept {
                count: kept.len() as u32,
                paths: kept,
            });
        }
        modpack::save(&ctx.entry_dir, &pack)?;
        tracing::info!(
            entry = %ctx.entry_dir.display(),
            pack = %pack.name,
            version = %pack.version_number,
            files = pack.files.len(),
            overrides = pack.overrides.len(),
            failures = failures.len(),
            "modpack installed"
        );
        Ok((pack, failures, warnings))
    }

    /// Download the pack's managed-kind files into the entry's managed
    /// directories and mirror them into `data/`. Each is recorded with the ids
    /// its download URL carries, so a pack's mods are ordinary tracked items —
    /// listable, updatable and update-checkable — rather than anonymous jars.
    async fn fetch_pack_content(
        &self,
        ctx: &EntryContent,
        files: &[(ContentKind, &ModpackFile)],
        resolved: &ResolvedModpack,
        job: &Job<'_>,
    ) -> (Vec<InstalledContent>, Vec<ContentFailure>) {
        let mut refs = self.identify(&resolved.source, files);
        let catalogue = self.hydrate(&resolved.source, &mut refs).await;

        let mut items = Vec::new();
        let mut failures = Vec::new();
        let total = files.len() as u64;
        for (done, (kind, file)) in files.iter().enumerate() {
            if job.check().is_err() {
                break;
            }
            let reference = refs.remove(file.path.as_str());
            let identity = FileIdentity {
                project: reference
                    .as_ref()
                    .and_then(|r| catalogue.projects.get(r.project_id.as_str()))
                    .cloned(),
                version: reference
                    .as_ref()
                    .and_then(|r| catalogue.versions.get(r.version_id.as_str()))
                    .cloned()
                    .unwrap_or_default(),
                reference,
            };
            let label = identity.label(file);
            let relabel = |p: &ProvisionProgress| {
                let mut progress = p.clone();
                progress.detail = label.clone();
                progress.item = done as u64 + 1;
                progress.items = total;
                job.report(&progress);
            };
            let labeled = Job::new(&relabel, job.cancel());
            labeled.report(&phase_progress(ProvisionPhase::Content));

            match self
                .install_pack_file(ctx, *kind, file, &resolved.source, &identity, &labeled)
                .await
            {
                Ok(item) => items.push(item),
                Err(e) => failures.push(ContentFailure {
                    item: file.path.clone(),
                    title: label,
                    error: crate::error_info(e),
                }),
            }
        }
        (items, failures)
    }

    async fn install_pack_file(
        &self,
        ctx: &EntryContent,
        kind: ContentKind,
        file: &ModpackFile,
        source: &str,
        identity: &FileIdentity,
        job: &Job<'_>,
    ) -> Result<InstalledContent> {
        // A pack may name a file its own platform publishes no download for —
        // CurseForge lets an author opt out of third-party distribution, and a
        // pack listing one is the common case, not an odd one. It is a per-file
        // failure naming what to fetch by hand, never a failed install.
        if file.artifact.url.is_empty() {
            bail!(ErrorInfo::ContentDownloadBlocked {
                title: identity.label(file),
                source: source.to_string(),
            });
        }
        materialize::validate_filename(&file.artifact.filename)?;
        let managed = content_target(ctx, kind, &file.artifact.filename)?;
        materialize::ensure_artifact(
            Some(&self.cache),
            &file.artifact,
            &managed,
            ProvisionPhase::Content,
            job,
        )
        .await?;

        let reference = identity.reference.as_ref();
        let project = identity.project.as_ref();
        let item = InstalledContent {
            kind,
            // A file the source does not recognise came from somewhere else;
            // recording it as that source's would let `update` re-pin it to a
            // project it never belonged to.
            source: match reference {
                Some(_) => source.to_string(),
                None => "file".to_string(),
            },
            project_id: reference.map(|r| r.project_id.clone()).unwrap_or_default(),
            slug: project.map(|p| p.slug.clone()).unwrap_or_default(),
            title: identity.label(file),
            version_id: reference.map(|r| r.version_id.clone()).unwrap_or_default(),
            version_number: identity.version.clone(),
            filename: file.artifact.filename.clone(),
            sha1: file
                .artifact
                .checksum
                .as_ref()
                .map(|c| c.hex.clone())
                .unwrap_or_default(),
            url: file.artifact.url.clone(),
            icon_url: project.map(|p| p.icon_url.clone()).unwrap_or_default(),
            installed_unix: registry::now_unix(),
            worlds: Vec::new(),
            origin: String::new(),
            enabled: true,
            disabled_worlds: Vec::new(),
        };
        install::apply_files(&ctx.entry_dir, &ctx.data_dir, &item, &ctx.worlds())?;
        Ok(item)
    }

    /// The index files that are not managed-kind content — a pack shipping its
    /// own `config/…` by URL rather than in `overrides/`. They go straight into
    /// the game directory and are recorded like any other pack-owned file.
    async fn fetch_loose_files(
        &self,
        ctx: &EntryContent,
        files: &[&ModpackFile],
        previous: Option<&InstalledModpack>,
        job: &Job<'_>,
    ) -> Vec<ModpackOverride> {
        let owned: HashMap<&str, &ModpackOverride> = previous
            .map(|p| p.overrides.iter().map(|o| (o.path.as_str(), o)).collect())
            .unwrap_or_default();
        let mut out = Vec::new();
        for file in files {
            if job.check().is_err() {
                break;
            }
            let target = match materialize::safe_join(&ctx.data_dir, &file.path) {
                Ok(path) => path,
                Err(e) => {
                    tracing::warn!(path = %file.path, error = %e, "unsafe modpack file path");
                    continue;
                }
            };
            let writable = match owned.get(file.path.as_str()) {
                Some(previous) => modpack::ours(&ctx.data_dir, previous),
                None => !target.exists(),
            };
            if !writable {
                continue;
            }
            if let Err(e) = materialize::ensure_artifact(
                Some(&self.cache),
                &file.artifact,
                &target,
                ProvisionPhase::Overrides,
                job,
            )
            .await
            {
                tracing::warn!(path = %file.path, error = %e, "cannot fetch a modpack file");
                continue;
            }
            let sha1 = file
                .artifact
                .checksum
                .as_ref()
                .map(|c| c.hex.clone())
                .or_else(|| install::sha1_file(&target).ok())
                .unwrap_or_default();
            out.push(ModpackOverride {
                path: file.path.clone(),
                sha1,
            });
        }
        out
    }

    /// Fold the pack's items into the entry's content index, tagged with the
    /// pack's origin, and drop the previous pack's items that this one no
    /// longer ships. A user re-installing one of them by hand clears the tag and
    /// takes ownership, so a tagged item is only ever removed here.
    fn merge_pack_items(
        &self,
        ctx: &EntryContent,
        pack: &InstalledModpack,
        items: Vec<InstalledContent>,
        previous: Option<&InstalledModpack>,
    ) -> Result<()> {
        let origin = modpack::origin(pack);
        let keeping: HashSet<&str> = pack.files.iter().map(String::as_str).collect();
        let worlds = ctx.worlds();
        let mut index = install::load(&ctx.entry_dir);

        if let Some(previous) = previous {
            let previous_origin = modpack::origin(previous);
            let dropped: Vec<InstalledContent> = index
                .iter()
                .filter(|i| i.origin == previous_origin && !keeping.contains(i.filename.as_str()))
                .cloned()
                .collect();
            for item in &dropped {
                install::remove_files(&ctx.entry_dir, &ctx.data_dir, item, &worlds);
            }
            index.retain(|i| !dropped.iter().any(|d| d.filename == i.filename));
        }

        for mut item in items {
            item.origin = origin.clone();
            match index.iter().position(|i| {
                i.kind == item.kind
                    && (i.filename == item.filename
                        || (!item.project_id.is_empty() && i.project_id == item.project_id))
            }) {
                Some(pos) => {
                    let old = index.remove(pos);
                    if old.filename != item.filename {
                        install::remove_files(&ctx.entry_dir, &ctx.data_dir, &old, &worlds);
                    }
                    // A user-installed copy keeps its own (untagged) ownership:
                    // the pack supplying the same file does not take it over.
                    if old.origin.is_empty() {
                        item.origin = String::new();
                    }
                    index.push(item);
                }
                None => index.push(item),
            }
        }
        install::save(&ctx.entry_dir, index)
    }
}

/// The kind a pack file's path names, when it is exactly one of the managed
/// load directories' own files. A nested path under one of them (or any other
/// directory) is a game-directory file instead — the managed dirs are flat.
pub(super) fn managed_kind(path: &str) -> Option<ContentKind> {
    let mut parts = path.split('/');
    let dir = parts.next()?;
    let file = parts.next()?;
    if file.is_empty() || parts.next().is_some() {
        return None;
    }
    match dir {
        "mods" => Some(ContentKind::Mod),
        "plugins" => Some(ContentKind::Plugin),
        "resourcepacks" => Some(ContentKind::ResourcePack),
        "shaderpacks" => Some(ContentKind::Shader),
        _ => None,
    }
}

/// The previous pack's game-directory files that the new one no longer ships.
pub(super) fn stale_overrides(
    previous: &InstalledModpack,
    current: &InstalledModpack,
) -> Vec<ModpackOverride> {
    let keeping: HashSet<&str> = current.overrides.iter().map(|o| o.path.as_str()).collect();
    previous
        .overrides
        .iter()
        .filter(|o| !keeping.contains(o.path.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_load_dir_file_is_managed_content() {
        assert_eq!(managed_kind("mods/sodium.jar"), Some(ContentKind::Mod));
        assert_eq!(
            managed_kind("plugins/essentials.jar"),
            Some(ContentKind::Plugin)
        );
        assert_eq!(
            managed_kind("resourcepacks/cozy.zip"),
            Some(ContentKind::ResourcePack)
        );
        assert_eq!(
            managed_kind("shaderpacks/bsl.zip"),
            Some(ContentKind::Shader)
        );
    }

    #[test]
    fn anything_else_is_a_game_directory_file() {
        // The managed dirs are flat, so a nested path cannot be mirrored.
        assert_eq!(managed_kind("mods/1.21/sodium.jar"), None);
        assert_eq!(managed_kind("config/sodium.json"), None);
        assert_eq!(managed_kind("options.txt"), None);
        assert_eq!(managed_kind("mods"), None);
        assert_eq!(managed_kind("mods/"), None);
    }

    fn pack(overrides: &[&str]) -> InstalledModpack {
        InstalledModpack {
            overrides: overrides
                .iter()
                .map(|p| ModpackOverride {
                    path: p.to_string(),
                    sha1: "hash".to_string(),
                })
                .collect(),
            ..InstalledModpack::default()
        }
    }

    #[test]
    fn an_update_marks_only_the_files_the_new_pack_dropped() {
        let stale = stale_overrides(
            &pack(&["config/a.toml", "config/b.toml"]),
            &pack(&["config/a.toml", "config/c.toml"]),
        );
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].path, "config/b.toml");
    }
}
