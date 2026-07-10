//! The management half: install, list, remove, and update content inside a
//! server or an instance over the daemon.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use client::proto::content::{
    ContentAddItem, ContentAddSpec, ContentFailure, ContentKind, InstalledContent,
};
use client::Client;

use super::browse::search_pick;
use super::format::{kind_plural, source_label};
use super::EntryKind;
use crate::ui::{self, ProvisionReporter, View};

/// The per-entry management grammar shared by every installable kind under a
/// server or instance. The entry is fixed by the enclosing `server <name>` /
/// `instance <name>` context, so these verbs name only the item.
#[derive(Subcommand)]
pub enum ContentCmd {
    /// Install content (a project slug/id, a source URL, or a local --file)
    #[command(visible_alias = "install")]
    Add {
        /// Project slug/id or a source project URL (prompts to search when omitted)
        item: Option<String>,
        #[arg(long, help = "Pin a specific version (id or version number)")]
        version: Option<String>,
        #[arg(
            long,
            help = "Import a local file instead of a project",
            conflicts_with = "version"
        )]
        file: Option<String>,
        #[arg(long, help = "Override the stored filename (url/file installs)")]
        filename: Option<String>,
        #[arg(
            long,
            help = "For a datapack on an instance: a save world to install into (repeatable; prompts to pick when omitted)"
        )]
        world: Vec<String>,
    },
    /// Installed content of this kind
    #[command(visible_alias = "ls")]
    List,
    /// Uninstall content (prompts to pick when omitted)
    #[command(visible_alias = "rm")]
    Remove {
        /// Installed item (project id, slug, filename, or title)
        item: Option<String>,
    },
    /// Update platform content to its newest compatible version
    Update {
        /// One installed item, or every one of this kind when omitted
        item: Option<String>,
    },
}

/// `hestia <server|instance> <name> <kind> add|list|remove|update`.
pub async fn run_entry(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    reference: &str,
    cmd: ContentCmd,
) -> Result<()> {
    match cmd {
        ContentCmd::Add {
            item,
            version,
            file,
            filename,
            world,
        } => {
            add(
                client, entry, kind, reference, item, version, file, filename, world,
            )
            .await
        }
        ContentCmd::List => list(client, entry, kind, reference).await,
        ContentCmd::Remove { item } => remove(client, entry, kind, reference, item).await,
        ContentCmd::Update { item } => update(client, entry, kind, reference, item).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn add(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    reference: &str,
    item: Option<String>,
    version: Option<String>,
    file: Option<String>,
    filename: Option<String>,
    world: Vec<String>,
) -> Result<()> {
    let (id, name) = resolve_entry(client, entry, reference).await?;

    let mut add_item = ContentAddItem {
        filename: filename.unwrap_or_default(),
        version: version.unwrap_or_default(),
        ..ContentAddItem::default()
    };
    if let Some(path) = file {
        add_item.path = std::fs::canonicalize(&path)
            .with_context(|| format!("cannot read {path}"))?
            .to_string_lossy()
            .into_owned();
    } else {
        let reference = match item {
            Some(reference) => reference,
            None => search_pick(client, kind).await?,
        };
        if is_url(&reference) {
            add_item.url = reference;
        } else {
            add_item.project = reference;
        }
    }

    let worlds = if kind == ContentKind::DataPack && matches!(entry, EntryKind::Instance) {
        pick_worlds(client, &id, world).await?
    } else {
        Vec::new()
    };

    let spec = ContentAddSpec {
        kind,
        items: vec![add_item],
        worlds,
        ..ContentAddSpec::default()
    };
    let (installed, failures) = install_spec(client, entry, &id, spec).await?;
    show_install_report(&name, &installed, &failures)
}

/// Print what a batch installed and what failed; errors (a non-zero exit)
/// when nothing landed at all.
pub(super) fn show_install_report(
    name: &str,
    installed: &[InstalledContent],
    failures: &[ContentFailure],
) -> Result<()> {
    for content in installed {
        let where_ = match world_label(content) {
            Some(world) => format!("'{name}' ({world})"),
            None => format!("'{name}'"),
        };
        ui::show(View::line(format!(
            "installed {} ({}) into {where_}",
            content.title, content.filename
        )))?;
    }
    for failure in failures {
        let label = if failure.title.is_empty() {
            failure.item.clone()
        } else {
            failure.title.clone()
        };
        ui::show(View::note(format!("failed {label}: {}", failure.message)))?;
    }
    if installed.is_empty() {
        match failures.is_empty() {
            true => ui::show(View::note("nothing installed")),
            false => bail!("nothing installed"),
        }
    } else {
        Ok(())
    }
}

pub(super) async fn install_spec(
    client: &Client,
    entry: EntryKind,
    id: &str,
    spec: ContentAddSpec,
) -> Result<(Vec<InstalledContent>, Vec<ContentFailure>)> {
    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let result = match entry {
        EntryKind::Server => {
            client
                .server()
                .content_add(id, spec, move |p| progress.update(p))
                .await
        }
        EntryKind::Instance => {
            client
                .instance()
                .content_add(id, spec, move |p| progress.update(p))
                .await
        }
    };
    reporter.finish();
    Ok(result?)
}

async fn list(client: &Client, entry: EntryKind, kind: ContentKind, reference: &str) -> Result<()> {
    let (id, name) = resolve_entry(client, entry, reference).await?;
    let (items, untracked) = match entry {
        EntryKind::Server => client.server().content_list(&id, kind).await?,
        EntryKind::Instance => client.instance().content_list(&id, kind).await?,
    };
    if items.is_empty() && untracked.is_empty() {
        return ui::show(View::note(format!("no {} installed", kind_plural(kind))));
    }
    if !items.is_empty() {
        if kind == ContentKind::DataPack {
            let rows = items
                .iter()
                .map(|i| {
                    vec![
                        i.title.clone(),
                        i.version_number.clone(),
                        world_label(i).unwrap_or("-").to_string(),
                        source_label(i),
                    ]
                })
                .collect();
            ui::show(View::table(
                format!("{name} {}", kind_plural(kind)),
                ["NAME", "VERSION", "WORLD", "SOURCE"],
                rows,
            ))?;
        } else {
            let rows = items
                .iter()
                .map(|i| vec![i.title.clone(), i.version_number.clone(), source_label(i)])
                .collect();
            ui::show(View::table(
                format!("{name} {}", kind_plural(kind)),
                ["NAME", "VERSION", "SOURCE"],
                rows,
            ))?;
        }
    }
    if !untracked.is_empty() {
        ui::show(View::note(format!(
            "{} untracked file(s) in the game dir: {}",
            untracked.len(),
            untracked.join(", ")
        )))?;
    }
    Ok(())
}

async fn remove(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    reference: &str,
    item: Option<String>,
) -> Result<()> {
    let (id, name) = resolve_entry(client, entry, reference).await?;
    let item = match item {
        Some(item) => item,
        None => {
            let (items, _) = match entry {
                EntryKind::Server => client.server().content_list(&id, kind).await?,
                EntryKind::Instance => client.instance().content_list(&id, kind).await?,
            };
            pick_installed(items)?
        }
    };
    match entry {
        EntryKind::Server => client.server().content_remove(&id, kind, &item).await?,
        EntryKind::Instance => client.instance().content_remove(&id, kind, &item).await?,
    }
    ui::show(View::line(format!("removed '{item}' from '{name}'")))
}

async fn update(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    reference: &str,
    item: Option<String>,
) -> Result<()> {
    let (id, name) = resolve_entry(client, entry, reference).await?;
    let target = item.unwrap_or_default();

    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let updated = {
        let result = match entry {
            EntryKind::Server => {
                client
                    .server()
                    .content_update(&id, kind, &target, move |p| progress.update(p))
                    .await
            }
            EntryKind::Instance => {
                client
                    .instance()
                    .content_update(&id, kind, &target, move |p| progress.update(p))
                    .await
            }
        };
        reporter.finish();
        result?
    };

    if updated.is_empty() {
        return ui::show(View::note(format!("'{name}' is already up to date")));
    }
    for content in &updated {
        ui::show(View::line(format!(
            "updated {} to {}",
            content.title, content.version_number
        )))?;
    }
    Ok(())
}

fn pick_installed(items: Vec<InstalledContent>) -> Result<String> {
    if items.is_empty() {
        bail!("nothing is installed");
    }
    let labels: Vec<String> = items
        .iter()
        .map(|i| format!("{} ({})", i.title, i.version_number))
        .collect();
    let index = ui::select("select an item", &labels)?;
    let chosen = &items[index];
    Ok(if chosen.project_id.is_empty() {
        chosen.filename.clone()
    } else {
        chosen.project_id.clone()
    })
}

/// Choose the save world(s) for an instance datapack: the given `--world`
/// flags when present, otherwise an interactive multi-select over the
/// instance's worlds (which errors when stdin is not a terminal, so scripts
/// must pass `--world`).
async fn pick_worlds(client: &Client, id: &str, world: Vec<String>) -> Result<Vec<String>> {
    let flags: Vec<String> = world.into_iter().filter(|w| !w.is_empty()).collect();
    if !flags.is_empty() {
        return Ok(flags);
    }
    let worlds = client.instance().worlds(id).await?;
    if worlds.is_empty() {
        bail!("no save worlds yet — launch the instance and create a world first");
    }
    let picks = ui::multi_select("select world(s)", &worlds)
        .context("pass --world to name the world(s)")?;
    Ok(picks.into_iter().map(|i| worlds[i].clone()).collect())
}

/// A datapack's world shown as its folder name (the last path component of the
/// stored `saves/<world>`); `None` for content that has no world.
fn world_label(content: &InstalledContent) -> Option<&str> {
    let world = content.world.rsplit('/').next().unwrap_or(&content.world);
    (!world.is_empty()).then_some(world)
}

/// Resolve a known server/instance reference to its `(id, name)`.
async fn resolve_entry(
    client: &Client,
    entry: EntryKind,
    reference: &str,
) -> Result<(String, String)> {
    let entries: Vec<(String, String)> = match entry {
        EntryKind::Server => client
            .server()
            .list()
            .await?
            .into_iter()
            .map(|s| (s.id, s.name))
            .collect(),
        EntryKind::Instance => client
            .instance()
            .list()
            .await?
            .into_iter()
            .map(|i| (i.id, i.name))
            .collect(),
    };
    entries
        .into_iter()
        .find(|(id, name)| id == reference || name == reference)
        .with_context(|| format!("no {} matches '{reference}'", entry.noun()))
}

fn is_url(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}
