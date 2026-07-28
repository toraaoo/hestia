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
//!
//! This module is the per-side seam; [`resolve`] turns a reference into an
//! archive and [`apply`] puts it onto an entry.

mod apply;
mod resolve;

use anyhow::Result;
use proto::content::{
    ContentFailure, ContentProject, InstalledContent, ModpackFile, ResolvedModpack,
};
use proto::error::{EntryKind, ErrorInfo};
use proto::modpack::{InstalledModpack, ModpackRef, ModpackRemoveResult, ModpackTarget};
use proto::warning::WarningInfo;

use super::content::entry::{EntryContent, EntrySide};
use crate::cancel::Job;
use crate::content::provider::FileRef;
use crate::content::{install, modpack, mrpack};
use crate::engine::{Engine, ServerCreateSpec, ServerUpdateSpec};

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
    reference: Option<FileRef>,
    project: Option<ContentProject>,
    /// The version *number* behind `reference.version_id`; empty when the
    /// catalogue could not be reached.
    version: String,
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
        let entry = match target {
            ModpackTarget::Create { name } => {
                let flavor = self.pack_flavor(&resolved, EntrySide::Client)?;
                job.check()?;
                self.create_instance(
                    &pack_entry_name(name, &resolved),
                    &flavor,
                    &resolved.game_version,
                    resolved.loader_version.clone(),
                    &[],
                )
                .await?
                .id
            }
            ModpackTarget::Existing { entry } => entry.clone(),
        };

        let outcome = self
            .apply_to_instance(&entry, &resolved, project.as_ref(), &mut archive, job)
            .await;
        if outcome.is_err() && matches!(target, ModpackTarget::Create { .. }) {
            // A create that could not be filled leaves nothing, exactly as a
            // failed or cancelled plain create does.
            let _ = self.instances.remove(&entry);
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
        let entry = match target {
            ModpackTarget::Create { name } => {
                if !eula {
                    anyhow::bail!(ErrorInfo::EulaRequired);
                }
                let flavor = self.pack_flavor(&resolved, EntrySide::Server)?;
                job.check()?;
                self.provision_server(
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
                .await?
                .0
                .id
            }
            ModpackTarget::Existing { entry } => entry.clone(),
        };

        let outcome = self
            .apply_to_server(&entry, &resolved, project.as_ref(), &mut archive, job)
            .await;
        if outcome.is_err() && matches!(target, ModpackTarget::Create { .. }) {
            let _ = self.servers.remove(&entry);
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
        let current = require_pack(&ctx, EntryKind::Instance, &record.name)?;
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
        let current = require_pack(&ctx, EntryKind::Server, &record.name)?;
        let (resolved, project, mut archive) = self.fetch_update(&current, version, job).await?;

        if resolved.game_version != record.profile.game_version {
            self.update_server(
                ServerUpdateSpec {
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
        let pack = require_pack(&ctx, EntryKind::Instance, &record.name)?;
        remove_pack(&ctx, &pack)
    }

    pub fn remove_server_modpack(&self, reference: &str) -> Result<ModpackRemoveResult> {
        let (record, ctx) = self.server_content_ctx(reference)?;
        let pack = require_pack(&ctx, EntryKind::Server, &record.name)?;
        remove_pack(&ctx, &pack)
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
}

fn require_pack(ctx: &EntryContent, entry: EntryKind, name: &str) -> Result<InstalledModpack> {
    modpack::load(&ctx.entry_dir).ok_or_else(|| {
        ErrorInfo::ModpackNotInstalled {
            entry,
            name: name.to_string(),
        }
        .into()
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_takes_the_packs_name_only_when_none_was_given() {
        let resolved = ResolvedModpack {
            name: "Fabulously Optimized".to_string(),
            ..ResolvedModpack::default()
        };
        assert_eq!(pack_entry_name("", &resolved), "Fabulously Optimized");
        assert_eq!(pack_entry_name("  my pack ", &resolved), "my pack");
    }

    #[test]
    fn a_pack_with_no_name_of_its_own_takes_the_projects() {
        let project = ContentProject {
            title: "Cozy".to_string(),
            ..ContentProject::default()
        };
        assert_eq!(
            pack_display_name(&ResolvedModpack::default(), Some(&project)),
            "Cozy"
        );
        assert_eq!(
            pack_display_name(&ResolvedModpack::default(), None),
            "modpack"
        );
    }
}
