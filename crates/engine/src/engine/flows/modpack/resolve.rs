//! Turning a pack *reference* into something installable: which archive it is,
//! which flavor it needs, and what the catalogue knows about the files inside.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Result};
use proto::content::{
    ContentKind, ContentProject, ContentVersion, ModpackFile, ReleaseChannel, ResolvedModpack,
    VersionQuery,
};
use proto::error::{EntryKind, ErrorInfo};
use proto::minecraft::ProvisionPhase;
use proto::modpack::{InstalledModpack, ModpackRef};

use super::super::content::entry::{EntryContent, EntrySide};
use super::super::phase_progress;
use crate::cancel::Job;
use crate::content::pack;
use crate::content::provider::FileRef;
use crate::engine::Engine;

/// A pack ready to install: its manifest, the project detail it has when it came
/// from a catalogue, and the archive its `overrides/` live in.
pub(super) type FetchedPack = (ResolvedModpack, Option<ContentProject>, pack::Archive);

/// What the catalogue knows about the pack's files, looked up in bulk. Empty
/// where the source could not be reached — the install proceeds regardless.
#[derive(Default)]
pub(super) struct Catalogue {
    pub(super) projects: HashMap<String, ContentProject>,
    /// Version id → version number, so a pack's mod lists a version like every
    /// other installed item.
    pub(super) versions: HashMap<String, String>,
}

impl Engine {
    /// A pack can only be installed into an entry that already runs what it
    /// pins: both are baked into the entry's resolved profile, and neither can
    /// be changed in place (a version update stays within one flavor).
    pub(super) fn ensure_entry_matches(
        &self,
        ctx: &EntryContent,
        resolved: &ResolvedModpack,
    ) -> Result<()> {
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
    pub(super) fn pack_flavor(
        &self,
        resolved: &ResolvedModpack,
        side: EntrySide,
    ) -> Result<String> {
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
    /// `.mrpack` — into everything installing needs.
    pub(super) async fn fetch_pack(
        &self,
        reference: &ModpackRef,
        job: &Job<'_>,
    ) -> Result<FetchedPack> {
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
            let mut archive = pack::Archive::read(path)?;
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
        Ok((resolved, Some(project), pack::Archive::open(bytes)?))
    }

    /// The pack an update moves to: the same project at another (or the newest)
    /// published version.
    pub(super) async fn fetch_update(
        &self,
        current: &InstalledModpack,
        version: &str,
        job: &Job<'_>,
    ) -> Result<FetchedPack> {
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
        // A pin the listing did not carry may still be a version id — a URL
        // pins one directly, and a long-published pack's list is paged. Asking
        // for it by id is the same request either way, so it costs a miss.
        if picked.is_none() && !pin.is_empty() {
            let pinned = [pin.to_string()];
            if let Ok(found) = self.content.versions_by_id(source, &pinned).await {
                if let Some(version) = found.into_iter().next() {
                    return Ok(version.id);
                }
            }
        }
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
    pub(super) fn identify<'a>(
        &self,
        source: &str,
        files: &[(ContentKind, &'a ModpackFile)],
    ) -> HashMap<&'a str, FileRef> {
        let mut out = HashMap::new();
        for (_, file) in files {
            if let Some(parsed) = self.content.parse_file_url(source, &file.artifact.url) {
                out.insert(file.path.as_str(), parsed);
            }
        }
        out
    }

    /// Two bulk lookups for everything the pack's files belong to — versions by
    /// id for the version *numbers*, projects by id for the titles and icons —
    /// so the pool reads like ordinary installed content rather than a hundred
    /// bare filenames. Best-effort in both halves: a pack still installs when
    /// the catalogue is unreachable, it just reads less well.
    ///
    /// Versions are looked up first because a source may name only the file in
    /// its download URLs (CurseForge does): the file is what knows its project,
    /// so `refs` are completed from the answer before the projects are asked
    /// for.
    pub(super) async fn hydrate(
        &self,
        source: &str,
        refs: &mut HashMap<&str, FileRef>,
    ) -> Catalogue {
        let mut wanted: Vec<String> = refs
            .values()
            .map(|r| r.version_id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        wanted.sort();
        wanted.dedup();
        let found = match self.content.versions_by_id(source, &wanted).await {
            Ok(found) => found,
            Err(e) => {
                tracing::warn!(error = %e, count = wanted.len(), "cannot look up modpack versions");
                Vec::new()
            }
        };
        for reference in refs.values_mut() {
            if !reference.project_id.is_empty() {
                continue;
            }
            if let Some(version) = found.iter().find(|v| v.id == reference.version_id) {
                reference.project_id = version.project_id.clone();
            }
        }

        let mut projects: Vec<String> = refs
            .values()
            .map(|r| r.project_id.clone())
            .filter(|id| !id.is_empty())
            .collect();
        projects.sort();
        projects.dedup();

        Catalogue {
            projects: match projects.is_empty() {
                true => HashMap::new(),
                false => match self.content.projects(source, &projects).await {
                    Ok(found) => found.into_iter().map(|p| (p.id.clone(), p)).collect(),
                    Err(e) => {
                        tracing::warn!(error = %e, count = projects.len(), "cannot look up modpack projects");
                        HashMap::new()
                    }
                },
            },
            versions: found
                .into_iter()
                .map(|v| (v.id, v.version_number))
                .collect(),
        }
    }
}
