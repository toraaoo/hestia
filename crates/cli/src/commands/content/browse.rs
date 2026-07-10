//! The discovery half: `hestia <kind> search|info|versions` against a content
//! source, plus the interactive project picker `add` falls back to.

use anyhow::{bail, Result};
use clap::Subcommand;
use client::proto::content::{
    ContentKind, ContentProject, ContentVersion, SearchQuery, VersionQuery,
};
use client::Client;

use super::format::{channel_label, compact, kind_plural, side_label, truncate};
use super::{SortArg, PAGE};
use crate::commands::connect;
use crate::ui::{self, Spinner, View};

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
pub(super) async fn search_pick(client: &Client, kind: ContentKind) -> Result<String> {
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
