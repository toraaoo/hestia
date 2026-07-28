//! Per-entry content management: install from a platform project, a source page
//! URL, or a local file; list, remove, and update what is installed. The managed
//! directory under the entry root is the source of truth; `data/` holds a mirror.
//!
//! This module is the seam between the two sides and the work: every verb has a
//! `server_*` and an `instance_*` form that resolves its entry into one
//! [`entry::EntryContent`] and hands off. The work lives beside it — [`entry`]
//! for what an entry is and accepts, [`add`] for installing, [`manage`] for
//! everything already installed.

mod add;
pub(super) mod entry;
mod manage;

use anyhow::{bail, Context, Result};
use proto::content::{ContentAddSpec, ContentFailure, ContentKind, InstalledContent};

use self::entry::{EntryContent, EntrySide};
use self::manage::{list_content, remove_content, set_enabled};
use super::phase_progress;
use crate::content::{install, profiles};
use crate::engine::Engine;
use crate::instances::InstanceRecord;
use crate::minecraft::materialize::OnProgress;
use crate::servers::ServerRecord;

impl Engine {
    /// Install a batch of content into a server — each item a platform project,
    /// a direct URL, or a local file. Returns everything installed (items plus
    /// required dependencies) and, per item that could not be installed, a
    /// failure; the batch continues past them.
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
        // Content something else installed is owned by that thing: removing it
        // locally would silently reappear at the next apply or pack update, so
        // the removal is refused (there is no local-exclusion mechanism) and the
        // user is pointed at whatever owns it.
        let tagged = install::load(&ctx.entry_dir)
            .into_iter()
            .find(|i| i.kind == kind && install::matches(i, item) && !i.origin.is_empty());
        if let Some(tagged) = tagged {
            bail!(
                "'{}' was installed by {}; remove it there instead",
                tagged.title,
                origin_owner(&tagged.origin),
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

    pub(super) fn server_content_ctx(
        &self,
        reference: &str,
    ) -> Result<(ServerRecord, EntryContent)> {
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

    pub(super) fn instance_content_ctx(
        &self,
        reference: &str,
    ) -> Result<(InstanceRecord, EntryContent)> {
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
}

/// Who an `origin` tag says installed an item, for a refusal that names where
/// to go instead.
fn origin_owner(origin: &str) -> String {
    match origin.split_once(':') {
        Some(("modpack", name)) => format!("modpack '{name}'"),
        Some((_, name)) => format!("global profile '{name}'"),
        None => format!("'{origin}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_names_whatever_owns_the_item() {
        assert_eq!(origin_owner("profile:starter"), "global profile 'starter'");
        assert_eq!(origin_owner("modpack:1KVo5zza"), "modpack '1KVo5zza'");
    }
}
