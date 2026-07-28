//! The entry shape a content operation works against, and the rules that depend
//! only on it: what the entry accepts, which loader its version lookups filter
//! by, and where a managed file lands.

use std::path::PathBuf;

use anyhow::{bail, Result};
use proto::content::{ContentAddSpec, ContentKind, ContentProject, SideSupport};

use crate::content::install;
use crate::engine::Engine;

/// The entry-shape a content operation needs, independent of whether the entry
/// is a server or an instance.
pub(in crate::engine::flows) struct EntryContent {
    pub(in crate::engine::flows) entry_dir: PathBuf,
    pub(in crate::engine::flows) data_dir: PathBuf,
    pub(in crate::engine::flows) game_version: String,
    pub(in crate::engine::flows) flavor: String,
    pub(in crate::engine::flows) side: EntrySide,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::engine::flows) enum EntrySide {
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
    pub(in crate::engine::flows) fn worlds(&self) -> Vec<String> {
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
    pub(super) fn ensure_accepts(&self, ctx: &EntryContent, requested: ContentKind) -> Result<()> {
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

    pub(in crate::engine::flows) fn accepted_kinds(&self, ctx: &EntryContent) -> Vec<ContentKind> {
        let loads = match ctx.side {
            EntrySide::Server => self.minecraft().server_loads(&ctx.flavor),
            EntrySide::Client => self.minecraft().instance_loads(&ctx.flavor),
        };
        accepted_kinds(ctx.side, loads)
    }

    /// What a server of this flavor can take. Published on the entry views so a
    /// front-end renders the daemon's answer instead of keeping its own copy of
    /// the flavor table — the same no-drift rule the wire contracts follow.
    pub fn server_accepts(&self, flavor: &str) -> Vec<ContentKind> {
        accepted_kinds(EntrySide::Server, self.minecraft().server_loads(flavor))
    }

    pub fn instance_accepts(&self, flavor: &str) -> Vec<ContentKind> {
        accepted_kinds(EntrySide::Client, self.minecraft().instance_loads(flavor))
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

/// The loader filter a kind's version lookup needs: the entry's own loader for
/// whatever its flavor loads (a mod on fabric, a plugin on paper — Modrinth
/// names both by the flavor), and the `datapack` pseudo-loader for datapacks —
/// Modrinth types datapacks as mods carrying that loader, so the filter is what
/// selects the datapack file over a jar. Folia is filtered as `folia`, not
/// widened to `paper`: a plugin that never claimed Folia support deadlocks on
/// its regionised scheduler.
pub(super) fn content_loader(kind: ContentKind, flavor: &str) -> Option<String> {
    match kind {
        ContentKind::Mod | ContentKind::Plugin => Some(flavor.to_string()),
        ContentKind::DataPack => Some("datapack".to_string()),
        _ => None,
    }
}

/// Reject content the platform marks unsupported for the entry's side
/// (`Unknown` passes — the platform did not say). Datapacks are exempt: they
/// run on the server side of any world, including a client's integrated server,
/// so a source's client-side flag must not block installing one on an instance.
/// Judged by the *requested* kind — Modrinth types datapacks as mod projects,
/// so `project.kind` would miss the exemption.
pub(super) fn side_gate(
    requested: ContentKind,
    project: &ContentProject,
    side: EntrySide,
) -> Result<()> {
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
pub(super) fn datapack_worlds(ctx: &EntryContent, spec: &ContentAddSpec) -> Result<Vec<String>> {
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
pub(in crate::engine::flows) fn content_target(
    ctx: &EntryContent,
    kind: ContentKind,
    filename: &str,
) -> Result<PathBuf> {
    Ok(ctx.entry_dir.join(install::kind_dir(kind)?).join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    pub(super) fn ctx() -> EntryContent {
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
