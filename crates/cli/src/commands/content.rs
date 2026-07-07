//! Third-party content: the kind-first browse commands (`hestia mod search`,
//! `resourcepack info`, …) and the shared per-entry management grammar
//! (`hestia instance mod add|list|remove|update`, `server mod …`). Browsing
//! hits a content source directly; management installs into a server or
//! instance over the daemon. Every argument omitted on a terminal is asked for
//! interactively; piped invocations must pass it.

use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::{Subcommand, ValueEnum};
use client::proto::content::{
    ContentAddSpec, ContentKind, ContentProject, ContentVersion, InstalledContent, SearchQuery,
    SearchSort, VersionQuery,
};
use client::Client;

use crate::commands::connect;
use crate::ui::{self, ProvisionReporter, Spinner, View};

const PAGE: u32 = 20;

/// The browse grammar shared by every content kind (`mod`, `modpack`,
/// resourcepack, shader, datapack).
#[derive(Subcommand)]
pub enum BrowseCmd {
    /// Search a source (browses when the query is omitted)
    Search {
        /// Search terms
        query: Option<String>,
        #[arg(short, long, help = "Filter by loader (e.g. fabric)")]
        loader: Option<String>,
        #[arg(short = 'g', long = "game-version", help = "Filter by game version")]
        game_version: Option<String>,
        #[arg(short, long, help = "Filter by category (repeatable)")]
        category: Vec<String>,
        #[arg(short, long, value_enum, default_value_t = SortArg::Relevance, help = "Result ordering")]
        sort: SortArg,
        #[arg(short = 'S', long, help = "Content source (default: modrinth)")]
        source: Option<String>,
        #[arg(long, default_value_t = PAGE, help = "Results per page")]
        limit: u32,
        #[arg(long, default_value_t = 0, help = "Skip this many results")]
        offset: u32,
    },
    /// A project's details (accepts a slug or id)
    Info {
        /// Project slug or id
        project: String,
        #[arg(short = 'S', long, help = "Content source (default: modrinth)")]
        source: Option<String>,
    },
    /// A project's downloadable versions, newest first
    Versions {
        /// Project slug or id
        project: String,
        #[arg(short, long, help = "Filter by loader (e.g. fabric)")]
        loader: Option<String>,
        #[arg(short = 'g', long = "game-version", help = "Filter by game version")]
        game_version: Option<String>,
        #[arg(short = 'S', long, help = "Content source (default: modrinth)")]
        source: Option<String>,
    },
}

/// The per-entry management grammar shared by every installable kind under a
/// server or instance.
#[derive(Subcommand)]
pub enum ContentCmd {
    /// Install content (a project slug/id, a source URL, or a local --file)
    #[command(visible_alias = "install")]
    Add {
        /// Server/instance name or id (prompts when omitted)
        entry: Option<String>,
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
    },
    /// Installed content of this kind
    #[command(visible_alias = "ls")]
    List {
        /// Server/instance name or id (prompts when omitted)
        entry: Option<String>,
    },
    /// Uninstall content (prompts to pick when omitted)
    #[command(visible_alias = "rm")]
    Remove {
        /// Server/instance name or id (prompts when omitted)
        entry: Option<String>,
        /// Installed item (project id, slug, filename, or title)
        item: Option<String>,
    },
    /// Update platform content to its newest compatible version
    Update {
        /// Server/instance name or id (prompts when omitted)
        entry: Option<String>,
        /// One installed item, or every one of this kind when omitted
        item: Option<String>,
    },
}

/// A management command targets either a server or an instance; the two
/// facades share the same content methods, so this only picks which to call.
#[derive(Clone, Copy)]
pub enum EntryKind {
    Server,
    Instance,
}

impl EntryKind {
    fn noun(self) -> &'static str {
        match self {
            EntryKind::Server => "server",
            EntryKind::Instance => "instance",
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SortArg {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl From<SortArg> for SearchSort {
    fn from(sort: SortArg) -> Self {
        match sort {
            SortArg::Relevance => SearchSort::Relevance,
            SortArg::Downloads => SearchSort::Downloads,
            SortArg::Follows => SearchSort::Follows,
            SortArg::Newest => SearchSort::Newest,
            SortArg::Updated => SearchSort::Updated,
        }
    }
}

/// `hestia <kind> search|info|versions`.
pub async fn run_browse(kind: ContentKind, cmd: BrowseCmd) -> Result<()> {
    let client = connect().await?;
    match cmd {
        BrowseCmd::Search {
            query,
            loader,
            game_version,
            category,
            sort,
            source,
            limit,
            offset,
        } => {
            let base = SearchQuery {
                source: source.unwrap_or_default(),
                kind,
                query: query.unwrap_or_default(),
                loader,
                game_version,
                categories: category,
                sort: sort.into(),
                limit: limit.clamp(1, 100),
                offset,
            };
            search_pages(&client, base).await
        }
        BrowseCmd::Info { project, source } => {
            let detail = {
                let _spinner = Spinner::start("fetching project");
                client
                    .content()
                    .project(&source.unwrap_or_default(), &project)
                    .await?
            };
            show_project(&detail)
        }
        BrowseCmd::Versions {
            project,
            loader,
            game_version,
            source,
        } => {
            let versions = {
                let _spinner = Spinner::start("fetching versions");
                client
                    .content()
                    .versions(&VersionQuery {
                        source: source.unwrap_or_default(),
                        project,
                        loader,
                        game_version,
                    })
                    .await?
            };
            show_versions(versions)
        }
    }
}

/// `hestia sources`.
pub async fn run_sources() -> Result<()> {
    let client = connect().await?;
    let sources = client.content().sources().await?;
    let rows = sources
        .into_iter()
        .map(|s| vec![s.id, s.name])
        .collect::<Vec<_>>();
    ui::show(View::table("content sources", ["ID", "NAME"], rows))
}

/// `hestia <server|instance> <kind> add|list|remove|update`.
pub async fn run_entry(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    cmd: ContentCmd,
) -> Result<()> {
    match cmd {
        ContentCmd::Add {
            entry: reference,
            item,
            version,
            file,
            filename,
        } => {
            add(
                client, entry, kind, reference, item, version, file, filename,
            )
            .await
        }
        ContentCmd::List { entry: reference } => list(client, entry, kind, reference).await,
        ContentCmd::Remove {
            entry: reference,
            item,
        } => remove(client, entry, kind, reference, item).await,
        ContentCmd::Update {
            entry: reference,
            item,
        } => update(client, entry, kind, reference, item).await,
    }
}

#[allow(clippy::too_many_arguments)]
async fn add(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    reference: Option<String>,
    item: Option<String>,
    version: Option<String>,
    file: Option<String>,
    filename: Option<String>,
) -> Result<()> {
    let (id, name) = pick_entry(client, entry, reference).await?;

    let mut spec = ContentAddSpec {
        kind,
        filename: filename.unwrap_or_default(),
        version: version.unwrap_or_default(),
        ..ContentAddSpec::default()
    };
    if let Some(path) = file {
        spec.path = std::fs::canonicalize(&path)
            .with_context(|| format!("cannot read {path}"))?
            .to_string_lossy()
            .into_owned();
    } else {
        let reference = match item {
            Some(reference) => reference,
            None => search_pick(client, kind).await?,
        };
        if is_url(&reference) {
            spec.url = reference;
        } else {
            spec.project = reference;
        }
    }

    let reporter = Arc::new(ProvisionReporter::new());
    let progress = reporter.clone();
    let installed = {
        let facade_progress = progress;
        let result = match entry {
            EntryKind::Server => {
                client
                    .server()
                    .content_add(&id, spec, move |p| facade_progress.update(p))
                    .await
            }
            EntryKind::Instance => {
                client
                    .instance()
                    .content_add(&id, spec, move |p| facade_progress.update(p))
                    .await
            }
        };
        reporter.finish();
        result?
    };

    if installed.is_empty() {
        return ui::show(View::note("nothing installed"));
    }
    for content in &installed {
        ui::show(View::line(format!(
            "installed {} ({}) into '{name}'",
            content.title, content.filename
        )))?;
    }
    Ok(())
}

async fn list(
    client: &Client,
    entry: EntryKind,
    kind: ContentKind,
    reference: Option<String>,
) -> Result<()> {
    let (id, name) = pick_entry(client, entry, reference).await?;
    let (items, untracked) = match entry {
        EntryKind::Server => client.server().content_list(&id, kind).await?,
        EntryKind::Instance => client.instance().content_list(&id, kind).await?,
    };
    if items.is_empty() && untracked.is_empty() {
        return ui::show(View::note(format!("no {} installed", kind_plural(kind))));
    }
    if !items.is_empty() {
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
    reference: Option<String>,
    item: Option<String>,
) -> Result<()> {
    let (id, name) = pick_entry(client, entry, reference).await?;
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
    reference: Option<String>,
    item: Option<String>,
) -> Result<()> {
    let (id, name) = pick_entry(client, entry, reference).await?;
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

/// Page through search results: render each page, and on a terminal offer to
/// step forward/back until the user is done.
async fn search_pages(client: &Client, mut query: SearchQuery) -> Result<()> {
    loop {
        let result = {
            let _spinner = Spinner::start("searching");
            client.content().search(&query).await?
        };
        if result.hits.is_empty() {
            return ui::show(View::note("no results"));
        }
        let rows = result
            .hits
            .iter()
            .map(|h| {
                vec![
                    h.title.clone(),
                    h.slug.clone(),
                    compact(h.downloads),
                    truncate(&h.description, 60),
                ]
            })
            .collect();
        ui::show(View::table(
            "results",
            ["NAME", "SLUG", "DOWNLOADS", "DESCRIPTION"],
            rows,
        ))?;
        let shown_to = query.offset + result.hits.len() as u32;
        ui::show(View::note(format!(
            "showing {}–{shown_to} of {}",
            query.offset + 1,
            result.total
        )))?;

        let has_next = shown_to < result.total;
        let has_prev = query.offset > 0;
        if !ui::is_interactive() || (!has_next && !has_prev) {
            return Ok(());
        }
        let mut options = Vec::new();
        if has_next {
            options.push("next page".to_string());
        }
        if has_prev {
            options.push("previous page".to_string());
        }
        options.push("done".to_string());
        let choice = ui::select("browse", &options)?;
        match options[choice].as_str() {
            "next page" => query.offset = shown_to,
            "previous page" => query.offset = query.offset.saturating_sub(query.limit),
            _ => return Ok(()),
        }
    }
}

/// Search interactively and return the chosen project's slug — the picker for
/// `add` with no item given.
async fn search_pick(client: &Client, kind: ContentKind) -> Result<String> {
    let query = ui::input(&format!("search {}", kind_plural(kind)), "")?;
    let result = {
        let _spinner = Spinner::start("searching");
        client
            .content()
            .search(&SearchQuery {
                kind,
                query,
                limit: PAGE,
                ..SearchQuery::default()
            })
            .await?
    };
    if result.hits.is_empty() {
        bail!("no results");
    }
    let labels: Vec<String> = result
        .hits
        .iter()
        .map(|h| format!("{} ({} downloads)", h.title, compact(h.downloads)))
        .collect();
    let index = ui::select("select a project", &labels)?;
    Ok(result.hits[index].slug.clone())
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

/// Resolve the entry to `(id, name)`, prompting with a selector when omitted.
async fn pick_entry(
    client: &Client,
    entry: EntryKind,
    provided: Option<String>,
) -> Result<(String, String)> {
    let mut entries: Vec<(String, String, String)> = match entry {
        EntryKind::Server => client
            .server()
            .list()
            .await?
            .into_iter()
            .map(|s| (s.id, s.name, format!("{} {}", s.flavor, s.game_version)))
            .collect(),
        EntryKind::Instance => client
            .instance()
            .list()
            .await?
            .into_iter()
            .map(|i| (i.id, i.name, format!("{} {}", i.flavor, i.game_version)))
            .collect(),
    };
    if entries.is_empty() {
        bail!("no {}s yet", entry.noun());
    }
    if let Some(reference) = provided {
        return entries
            .into_iter()
            .find(|(id, name, _)| *id == reference || *name == reference)
            .map(|(id, name, _)| (id, name))
            .with_context(|| format!("no {} matches '{reference}'", entry.noun()));
    }
    let labels: Vec<String> = entries
        .iter()
        .map(|(_, name, detail)| format!("{name} ({detail})"))
        .collect();
    let index = ui::select(&format!("select a {}", entry.noun()), &labels)?;
    let (id, name, _) = entries.swap_remove(index);
    Ok((id, name))
}

fn show_project(project: &ContentProject) -> Result<()> {
    ui::show(View::detail([
        ("title", project.title.clone()),
        ("slug", project.slug.clone()),
        ("id", project.id.clone()),
        ("source", project.source.clone()),
        ("downloads", compact(project.downloads)),
        ("follows", compact(project.follows)),
        ("client", side_label(project.client_side)),
        ("server", side_label(project.server_side)),
        ("categories", project.categories.join(", ")),
        ("description", project.description.clone()),
    ]))
}

fn show_versions(versions: Vec<ContentVersion>) -> Result<()> {
    if versions.is_empty() {
        return ui::show(View::note("no versions"));
    }
    let rows = versions
        .iter()
        .map(|v| {
            vec![
                v.version_number.clone(),
                channel_label(v.channel).to_string(),
                v.game_versions.join(", "),
                v.loaders.join(", "),
            ]
        })
        .collect();
    ui::show(View::table(
        "versions",
        ["VERSION", "CHANNEL", "GAME", "LOADERS"],
        rows,
    ))
}

fn is_url(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}

fn source_label(item: &InstalledContent) -> String {
    if item.project_id.is_empty() {
        item.source.clone()
    } else {
        format!("{} ({})", item.source, item.version_number)
    }
}

fn side_label(side: client::proto::content::SideSupport) -> String {
    use client::proto::content::SideSupport::*;
    match side {
        Required => "required",
        Optional => "optional",
        Unsupported => "unsupported",
        Unknown => "unknown",
    }
    .to_string()
}

fn channel_label(channel: client::proto::content::ReleaseChannel) -> &'static str {
    use client::proto::content::ReleaseChannel::*;
    match channel {
        Release => "release",
        Beta => "beta",
        Alpha => "alpha",
    }
}

fn kind_plural(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mods",
        ContentKind::Modpack => "modpacks",
        ContentKind::ResourcePack => "resourcepacks",
        ContentKind::Shader => "shaders",
        ContentKind::DataPack => "datapacks",
    }
}

/// A large count in compact units (180204729 → "180.2M").
fn compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn truncate(text: &str, max: usize) -> String {
    let flat = text.replace('\n', " ");
    if flat.chars().count() <= max {
        return flat;
    }
    let mut out: String = flat.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
