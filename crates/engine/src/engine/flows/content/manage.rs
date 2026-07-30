//! What happens to content already in the pool: listing it, moving it to
//! another version, checking whether one exists, toggling it, and removing it.

use anyhow::{bail, Context, Result};
use proto::content::{ContentKind, ContentProject, InstalledContent, UntrackedFile, VersionQuery};
use proto::minecraft::ProvisionPhase;

use super::entry::{content_loader, EntryContent};
use super::phase_progress;
use crate::content::install;
use crate::engine::Engine;
use crate::minecraft::materialize::OnProgress;

impl Engine {
    /// Move matched platform items to a newer version — the newest compatible
    /// when `pin` is empty, or that exact version (id or number) when pinned.
    pub(super) async fn update_content(
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

    pub(super) async fn content_updates(
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
}

pub(super) fn list_content(
    ctx: &EntryContent,
    kind: ContentKind,
) -> (Vec<InstalledContent>, Vec<UntrackedFile>) {
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
pub(super) fn remove_content(
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
pub(super) fn set_enabled(
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
}
