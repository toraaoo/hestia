//! Modpack install, update and removal, for a new or an existing entry.
//!
//! A pack is three different things at once, and each goes where it belongs
//! rather than into a parallel store of its own:
//!
//! - its **loader and game version** become the entry's flavor and version, so
//!   creating from a pack is the ordinary create with those filled in;
//! - its **index files under a managed kind directory** become ordinary pool
//!   items tagged `modpack:<project>`, so the launch-time mirror, the backup
//!   heal, `content list` and per-item update all work on them unchanged;
//! - everything else it ships — its `overrides/`, plus any index file outside a
//!   managed kind directory — is written straight into the game directory and
//!   recorded in `modpack.json` with the hash it was written with.
//!
//! Only the third needs a record, and only because those files are the ones the
//! user edits: the hash is what lets an update replace a file the pack still
//! owns while leaving a tweaked one exactly as it was found.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{bail, Result};
use proto::content::{
    ContentFailure, ContentKind, ContentProject, ContentVersion, InstalledContent, ModpackFile,
    ReleaseChannel, ResolvedModpack, VersionQuery,
};
use proto::error::{EntryKind, ErrorInfo};
use proto::minecraft::{ProvisionPhase, ProvisionProgress};
use proto::modpack::{
    InstalledModpack, ModpackOverride, ModpackRef, ModpackRemoveResult, ModpackTarget,
};
use proto::warning::WarningInfo;

use super::content::{content_target, EntryContent, EntrySide};
use super::phase_progress;
use crate::cancel::Job;
use crate::content::{install, modpack, mrpack};
use crate::engine::{Engine, ServerCreateSpec};
use crate::minecraft::materialize;
use crate::registry;

/// What a finished install or update produced. `entry` is the entry's id —
/// which, for a create, the caller has no other way to learn.
pub struct ModpackOutcome {
    pub entry: String,
    pub entry_name: String,
    pub pack: InstalledModpack,
    pub failures: Vec<ContentFailure>,
    pub warnings: Vec<WarningInfo>,
}

/// What the catalogue could be told about one pack file. A pack names its files
/// by URL and hash alone, so both halves are absent for a file the source does
/// not serve — and that file installs anyway, just untracked.
struct FileIdentity {
    reference: Option<crate::content::provider::FileRef>,
    project: Option<ContentProject>,
}

impl FileIdentity {
    /// What to call the file while it downloads and in the pool: the project's
    /// title where there is one, the filename otherwise.
    fn label(&self, file: &ModpackFile) -> String {
        self.project
            .as_ref()
            .map(|p| p.title.clone())
            .unwrap_or_else(|| file.artifact.filename.clone())
    }
}

impl Engine {
    /// Install a pack into a new or existing instance.
    pub async fn install_instance_modpack(
        &self,
        reference: &ModpackRef,
        target: &ModpackTarget,
        job: &Job<'_>,
    ) -> Result<ModpackOutcome> {
        let (resolved, project, mut archive) = self.fetch_pack(reference, job).await?;
        let name = match target {
            ModpackTarget::Create { name } => {
                let flavor = self.pack_flavor(&resolved, EntrySide::Client)?;
                job.check()?;
                let record = self
                    .create_instance(
                        &pack_entry_name(name, &resolved),
                        &flavor,
                        &resolved.game_version,
                        resolved.loader_version.clone(),
                        &[],
                    )
                    .await?;
                record.id
            }
            ModpackTarget::Existing { entry } => entry.clone(),
        };

        let created = matches!(target, ModpackTarget::Create { .. });
        let outcome = self
            .apply_to_instance(&name, &resolved, project.as_ref(), &mut archive, job)
            .await;
        if outcome.is_err() && created {
            // A create that could not be filled leaves nothing, exactly as a
            // failed or cancelled plain create does.
            let _ = self.instances.remove(&name);
        }
        outcome
    }

    /// Install a pack into a new or existing server. A pack is client-shaped by
    /// convention, so the server side takes what the pack marks as its own —
    /// `env.server`, plus `server-overrides/` in place of `client-overrides/`.
    pub async fn install_server_modpack(
        &self,
        reference: &ModpackRef,
        target: &ModpackTarget,
        eula: bool,
        port: Option<u16>,
        job: &Job<'_>,
    ) -> Result<ModpackOutcome> {
        let (resolved, project, mut archive) = self.fetch_pack(reference, job).await?;
        let name = match target {
            ModpackTarget::Create { name } => {
                if !eula {
                    bail!(ErrorInfo::EulaRequired);
                }
                let flavor = self.pack_flavor(&resolved, EntrySide::Server)?;
                job.check()?;
                let (record, _) = self
                    .provision_server(
                        ServerCreateSpec {
                            name: pack_entry_name(name, &resolved),
                            flavor,
                            version: resolved.game_version.clone(),
                            loader_version: resolved.loader_version.clone(),
                            port,
                            config: Vec::new(),
                        },
                        job,
                    )
                    .await?;
                record.id
            }
            ModpackTarget::Existing { entry } => entry.clone(),
        };

        let created = matches!(target, ModpackTarget::Create { .. });
        let outcome = self
            .apply_to_server(&name, &resolved, project.as_ref(), &mut archive, job)
            .await;
        if outcome.is_err() && created {
            let _ = self.servers.remove(&name);
        }
        outcome
    }

    /// Move an instance's pack to another published version. A pack update
    /// carries the game version with it — that is what updating a pack means —
    /// so the entry's own version moves too, behind the same downgrade gate the
    /// plain version update uses.
    pub async fn update_instance_modpack(
        &self,
        reference: &str,
        version: &str,
        allow_downgrade: bool,
        job: &Job<'_>,
    ) -> Result<ModpackOutcome> {
        let (record, ctx) = self.instance_content_ctx(reference)?;
        let current = self.require_pack(&ctx, EntryKind::Instance, &record.name)?;
        let (resolved, project, mut archive) = self.fetch_update(&current, version, job).await?;

        if resolved.game_version != record.profile.game_version {
            self.update_instance(
                &record.id,
                &resolved.game_version,
                resolved.loader_version.clone(),
                allow_downgrade,
            )
            .await?;
        }
        self.apply_to_instance(&record.id, &resolved, project.as_ref(), &mut archive, job)
            .await
    }

    pub async fn update_server_modpack(
        &self,
        reference: &str,
        version: &str,
        allow_downgrade: bool,
        job: &Job<'_>,
    ) -> Result<ModpackOutcome> {
        let (record, ctx) = self.server_content_ctx(reference)?;
        let current = self.require_pack(&ctx, EntryKind::Server, &record.name)?;
        let (resolved, project, mut archive) = self.fetch_update(&current, version, job).await?;

        if resolved.game_version != record.profile.game_version {
            self.update_server(
                crate::engine::ServerUpdateSpec {
                    server: record.id.clone(),
                    version: resolved.game_version.clone(),
                    loader_version: resolved.loader_version.clone(),
                    allow_downgrade,
                },
                job,
            )
            .await?;
        }
        self.apply_to_server(&record.id, &resolved, project.as_ref(), &mut archive, job)
            .await
    }

    pub fn instance_modpack(&self, reference: &str) -> Result<Option<InstalledModpack>> {
        let (_, ctx) = self.instance_content_ctx(reference)?;
        Ok(modpack::load(&ctx.entry_dir))
    }

    pub fn server_modpack(&self, reference: &str) -> Result<Option<InstalledModpack>> {
        let (_, ctx) = self.server_content_ctx(reference)?;
        Ok(modpack::load(&ctx.entry_dir))
    }

    pub fn remove_instance_modpack(&self, reference: &str) -> Result<ModpackRemoveResult> {
        let (record, ctx) = self.instance_content_ctx(reference)?;
        let pack = self.require_pack(&ctx, EntryKind::Instance, &record.name)?;
        remove_pack(&ctx, &pack)
    }

    pub fn remove_server_modpack(&self, reference: &str) -> Result<ModpackRemoveResult> {
        let (record, ctx) = self.server_content_ctx(reference)?;
        let pack = self.require_pack(&ctx, EntryKind::Server, &record.name)?;
        remove_pack(&ctx, &pack)
    }

    fn require_pack(
        &self,
        ctx: &EntryContent,
        entry: EntryKind,
        name: &str,
    ) -> Result<InstalledModpack> {
        modpack::load(&ctx.entry_dir).ok_or_else(|| {
            ErrorInfo::ModpackNotInstalled {
                entry,
                name: name.to_string(),
            }
            .into()
        })
    }

    async fn apply_to_instance(
        &self,
        reference: &str,
        resolved: &ResolvedModpack,
        project: Option<&ContentProject>,
        archive: &mut mrpack::Archive,
        job: &Job<'_>,
    ) -> Result<ModpackOutcome> {
        let (record, ctx) = self.instance_content_ctx(reference)?;
        let (pack, failures, warnings) = self
            .apply_pack(&ctx, resolved, project, archive, job)
            .await?;
        Ok(ModpackOutcome {
            entry: record.id,
            entry_name: record.name,
            pack,
            failures,
            warnings,
        })
    }

    async fn apply_to_server(
        &self,
        reference: &str,
        resolved: &ResolvedModpack,
        project: Option<&ContentProject>,
        archive: &mut mrpack::Archive,
        job: &Job<'_>,
    ) -> Result<ModpackOutcome> {
        let (record, ctx) = self.server_content_ctx(reference)?;
        let (pack, failures, warnings) = self
            .apply_pack(&ctx, resolved, project, archive, job)
            .await?;
        Ok(ModpackOutcome {
            entry: record.id,
            entry_name: record.name,
            pack,
            failures,
            warnings,
        })
    }

    /// The install itself, once the entry exists: refuse a pack the entry
    /// cannot run, download its managed files, write its game-directory files,
    /// and reconcile against whatever pack was there before.
    async fn apply_pack(
        &self,
        ctx: &EntryContent,
        resolved: &ResolvedModpack,
        project: Option<&ContentProject>,
        archive: &mut mrpack::Archive,
        job: &Job<'_>,
    ) -> Result<(InstalledModpack, Vec<ContentFailure>, Vec<WarningInfo>)> {
        self.ensure_entry_matches(ctx, resolved)?;
        let previous = modpack::load(&ctx.entry_dir);
        let side = match ctx.side {
            EntrySide::Server => mrpack::Side::Server,
            EntrySide::Client => mrpack::Side::Client,
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
            let keep = match owned.get(path) {
                // Known to the previous pack: only overwrite what is still
                // byte-for-byte what we wrote.
                Some(previous) => modpack::ours(&data_dir, previous),
                // Unknown: an existing file was not put there by a pack, so it
                // is the user's (or the game's) and is not ours to replace.
                None => !data_dir.join(path).exists(),
            };
            if !keep {
                kept.push(path.to_string());
            }
            keep
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
            warnings.push(WarningInfo::ModpackOverridesKept { paths: kept });
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
        let titles = self.hydrate(&resolved.source, &refs).await;

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
                    .and_then(|r| titles.get(r.project_id.as_str()))
                    .cloned(),
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
        if file.artifact.url.is_empty() {
            bail!(ErrorInfo::FieldRequired {
                field: proto::error::Field::Url
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
            version_number: String::new(),
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

    /// A pack can only be installed into an entry that already runs what it
    /// pins: both are baked into the entry's resolved profile, and neither can
    /// be changed in place (a version update stays within one flavor).
    fn ensure_entry_matches(&self, ctx: &EntryContent, resolved: &ResolvedModpack) -> Result<()> {
        let side = ctx.side;
        let pack_flavor = self.pack_flavor(resolved, side)?;
        if pack_flavor == ctx.flavor && resolved.game_version == ctx.game_version {
            return Ok(());
        }
        bail!(ErrorInfo::ModpackEntryMismatch {
            entry: match side {
                EntrySide::Server => EntryKind::Server,
                EntrySide::Client => EntryKind::Instance,
            },
            name: entry_name(&ctx.entry_dir),
            flavor: ctx.flavor.clone(),
            game_version: ctx.game_version.clone(),
            pack_flavor,
            pack_game_version: resolved.game_version.clone(),
        })
    }

    /// The flavor a pack's loader names. The registry is the table: a pack's
    /// loader name *is* hestia's flavor id where the flavor exists, so adding a
    /// flavor needs no edit here — and a pack pinning one that does not exist is
    /// refused by name rather than silently installed as vanilla.
    fn pack_flavor(&self, resolved: &ResolvedModpack, side: EntrySide) -> Result<String> {
        let flavors = match side {
            EntrySide::Server => self.minecraft.server_flavors(),
            EntrySide::Client => self.minecraft.instance_flavors(),
        };
        let wanted = resolved
            .loader
            .as_deref()
            .filter(|l| !l.is_empty())
            .unwrap_or("vanilla");
        match flavors.iter().any(|f| f.id == wanted) {
            true => Ok(wanted.to_string()),
            false => bail!(ErrorInfo::ModpackLoaderUnsupported {
                loader: wanted.to_string()
            }),
        }
    }

    /// Resolve a pack reference — a project, a source page URL, or a local
    /// `.mrpack` — into its manifest, its project detail where it has one, and
    /// the archive its `overrides/` live in.
    async fn fetch_pack(
        &self,
        reference: &ModpackRef,
        job: &Job<'_>,
    ) -> Result<(ResolvedModpack, Option<ContentProject>, mrpack::Archive)> {
        let picked = [&reference.project, &reference.url, &reference.path]
            .iter()
            .filter(|s| !s.is_empty())
            .count();
        if picked != 1 {
            bail!(ErrorInfo::MutuallyExclusive {
                options: vec!["a project".into(), "a url".into(), "a file".into()],
            });
        }
        job.report(&phase_progress(ProvisionPhase::Resolving));

        if !reference.path.is_empty() {
            let path = Path::new(&reference.path);
            let mut archive = mrpack::Archive::read(path)?;
            let mut resolved = archive.index()?;
            resolved.source = "file".to_string();
            if resolved.name.is_empty() {
                resolved.name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
            return Ok((resolved, None, archive));
        }

        let (source, project_ref, pin) = match reference.url.is_empty() {
            false => {
                let (source, parsed) = self.content.parse_url(&reference.url).ok_or_else(|| {
                    ErrorInfo::UnsupportedContentUrl {
                        url: reference.url.clone(),
                    }
                })?;
                let pin = parsed.version.unwrap_or_else(|| reference.version.clone());
                (source, parsed.project, pin)
            }
            true => (
                reference.source.clone(),
                reference.project.clone(),
                reference.version.clone(),
            ),
        };

        let project = self
            .content
            .project(&source, &project_ref, Some(ContentKind::Modpack))
            .await?;
        if !project.kinds.contains(&ContentKind::Modpack) && project.kind != ContentKind::Modpack {
            bail!(ErrorInfo::ContentKindMismatch {
                title: project.title.clone(),
                actual: project.kind,
                expected: ContentKind::Modpack,
            });
        }
        let version = self.pick_pack_version(&source, &project.id, &pin).await?;
        job.check()?;
        let (resolved, bytes) = self.content.fetch_modpack(&source, &version).await?;
        Ok((resolved, Some(project), mrpack::Archive::open(bytes)?))
    }

    /// The pack an update moves to: the same project at another (or the newest)
    /// published version.
    async fn fetch_update(
        &self,
        current: &InstalledModpack,
        version: &str,
        job: &Job<'_>,
    ) -> Result<(ResolvedModpack, Option<ContentProject>, mrpack::Archive)> {
        if current.project_id.is_empty() {
            bail!(ErrorInfo::UnsupportedOperation {
                reason: proto::error::Unsupported::ModpackNotUpdatable
            });
        }
        self.fetch_pack(
            &ModpackRef {
                source: current.source.clone(),
                project: current.project_id.clone(),
                version: version.to_string(),
                ..ModpackRef::default()
            },
            job,
        )
        .await
    }

    /// Newest-first, so an empty pin takes the newest release and falls back to
    /// the newest of any channel. Deliberately unfiltered by game version: a
    /// pack *states* its game version rather than being selected for one.
    async fn pick_pack_version(&self, source: &str, project: &str, pin: &str) -> Result<String> {
        let versions = self
            .content
            .versions(&VersionQuery {
                source: source.to_string(),
                project: project.to_string(),
                ..VersionQuery::default()
            })
            .await?;
        let picked: Option<&ContentVersion> = match pin.is_empty() {
            false => versions
                .iter()
                .find(|v| v.id == pin || v.version_number == pin),
            true => versions
                .iter()
                .find(|v| v.channel == ReleaseChannel::Release)
                .or_else(|| versions.first()),
        };
        picked.map(|v| v.id.clone()).ok_or_else(|| {
            ErrorInfo::VersionNotFound {
                reference: match pin.is_empty() {
                    true => project.to_string(),
                    false => pin.to_string(),
                },
            }
            .into()
        })
    }

    /// Which platform file each managed pack file is, from its download URL.
    fn identify<'a>(
        &self,
        source: &str,
        files: &[(ContentKind, &'a ModpackFile)],
    ) -> HashMap<&'a str, crate::content::provider::FileRef> {
        let mut out = HashMap::new();
        for (_, file) in files {
            if let Some(parsed) = self.content.parse_file_url(source, &file.artifact.url) {
                out.insert(file.path.as_str(), parsed);
            }
        }
        out
    }

    /// One bulk lookup for every project the pack's files belong to, so the pool
    /// shows titles and icons instead of a hundred bare filenames. Best-effort:
    /// a pack still installs when the catalogue is unreachable, it just reads
    /// less well.
    async fn hydrate(
        &self,
        source: &str,
        refs: &HashMap<&str, crate::content::provider::FileRef>,
    ) -> HashMap<String, ContentProject> {
        let mut ids: Vec<String> = refs.values().map(|r| r.project_id.clone()).collect();
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            return HashMap::new();
        }
        match self.content.projects(source, &ids).await {
            Ok(projects) => projects.into_iter().map(|p| (p.id.clone(), p)).collect(),
            Err(e) => {
                tracing::warn!(error = %e, count = ids.len(), "cannot look up modpack projects");
                HashMap::new()
            }
        }
    }
}

/// Take the pack out of an entry: its pool items and its game-directory files,
/// leaving anything the user has since edited. The entry itself stays — a pack
/// is content an entry carries, not its identity.
fn remove_pack(ctx: &EntryContent, pack: &InstalledModpack) -> Result<ModpackRemoveResult> {
    let origin = modpack::origin(pack);
    let worlds = ctx.worlds();
    let mut index = install::load(&ctx.entry_dir);
    let dropped: Vec<InstalledContent> = index
        .iter()
        .filter(|i| i.origin == origin)
        .cloned()
        .collect();
    for item in &dropped {
        install::remove_files(&ctx.entry_dir, &ctx.data_dir, item, &worlds);
    }
    index.retain(|i| i.origin != origin);
    install::save(&ctx.entry_dir, index)?;

    let (removed_overrides, kept) = modpack::remove_overrides(&ctx.data_dir, &pack.overrides);
    modpack::clear(&ctx.entry_dir);
    tracing::info!(
        entry = %ctx.entry_dir.display(),
        pack = %pack.name,
        files = dropped.len(),
        overrides = removed_overrides,
        kept = kept.len(),
        "modpack removed"
    );
    Ok(ModpackRemoveResult {
        removed_files: dropped.len() as u32,
        removed_overrides,
        kept,
    })
}

/// The kind a pack file's path names, when it is exactly one of the managed
/// load directories' own files. A nested path under one of them (or any other
/// directory) is a game-directory file instead — the managed dirs are flat.
fn managed_kind(path: &str) -> Option<ContentKind> {
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
fn stale_overrides(
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

fn pack_entry_name(given: &str, resolved: &ResolvedModpack) -> String {
    match given.trim().is_empty() {
        false => given.trim().to_string(),
        true => resolved.name.clone(),
    }
}

fn pack_display_name(resolved: &ResolvedModpack, project: Option<&ContentProject>) -> String {
    let from_project = project.map(|p| p.title.as_str()).unwrap_or_default();
    match (resolved.name.is_empty(), from_project.is_empty()) {
        (false, _) => resolved.name.clone(),
        (true, false) => from_project.to_string(),
        (true, true) => "modpack".to_string(),
    }
}

fn entry_name(entry_dir: &Path) -> String {
    entry_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
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

    #[test]
    fn an_entry_takes_the_packs_name_only_when_none_was_given() {
        let resolved = ResolvedModpack {
            name: "Fabulously Optimized".to_string(),
            ..ResolvedModpack::default()
        };
        assert_eq!(pack_entry_name("", &resolved), "Fabulously Optimized");
        assert_eq!(pack_entry_name("  my pack ", &resolved), "my pack");
    }
}
