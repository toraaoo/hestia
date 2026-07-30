//! `hestia news …` — the announcements the launcher fetches from its feed.
//!
//! Reads answer from the daemon's cache, so the list is instant and works
//! offline; `refresh` is the only verb that touches the network.

use anyhow::Result;
use clap::Subcommand;
use proto::announce::{AnnounceListResult, Announcement, Severity};

use crate::commands::mc;
use crate::ui::{self, View};

#[derive(Subcommand)]
pub enum NewsCmd {
    /// News and notices for this build (the default)
    #[command(alias = "ls")]
    List {
        /// Include announcements already marked read
        #[arg(long)]
        all: bool,
    },
    /// Print one announcement in full
    Show { id: String },
    /// Mark announcements read; with no ids, every unread one
    Read { ids: Vec<String> },
    /// Fetch the feed now instead of waiting for the daemon's poll
    Refresh,
}

pub async fn run(cmd: Option<NewsCmd>) -> Result<()> {
    let client = super::connect().await?;
    match cmd.unwrap_or(NewsCmd::List { all: false }) {
        NewsCmd::List { all } => {
            let result = client.announce().list().await?;
            list(result, all)
        }
        NewsCmd::Show { id } => {
            let result = client.announce().list().await?;
            show(result.announcements, &id)
        }
        NewsCmd::Read { ids } => {
            let result = client.announce().list().await?;
            let ids = if ids.is_empty() {
                unread(&result.announcements)
                    .map(|a| a.id.clone())
                    .collect()
            } else {
                ids
            };
            if ids.is_empty() {
                return ui::show(View::note("nothing unread"));
            }
            let count = ids.len();
            client.announce().dismiss(ids).await?;
            ui::show(View::line(format!("marked {count} read")))
        }
        NewsCmd::Refresh => {
            let result = client.announce().refresh().await?;
            list(result, false)
        }
    }
}

fn unread(announcements: &[Announcement]) -> impl Iterator<Item = &Announcement> {
    announcements.iter().filter(|a| !a.dismissed)
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Critical => "critical",
    }
}

fn list(result: AnnounceListResult, all: bool) -> Result<()> {
    if result.announcements.is_empty() {
        // Three different empties that render identically in a table: switched
        // off, never fetched, and nothing published. Say which.
        return ui::show(View::note(if !result.enabled {
            "announcements are off — `hestia config set announcements.enabled true` turns them on"
        } else if result.fetched == 0 {
            "no announcements — the feed has not been fetched yet"
        } else {
            "no announcements"
        }));
    }
    let announcements = result.announcements;
    let shown: Vec<&Announcement> = if all {
        announcements.iter().collect()
    } else {
        unread(&announcements).collect()
    };
    if shown.is_empty() {
        return ui::show(View::note(
            "nothing unread — `hestia news --all` shows every announcement",
        ));
    }
    let rows = shown
        .iter()
        .map(|a| {
            vec![
                a.id.clone(),
                severity_label(a.severity).to_string(),
                mc::age_label(a.published),
                if a.dismissed { "read" } else { "new" }.to_string(),
                a.title.clone(),
            ]
        })
        .collect();
    ui::show(View::table(
        "news",
        ["ID", "SEVERITY", "PUBLISHED", "STATUS", "TITLE"],
        rows,
    ))
}

fn show(announcements: Vec<Announcement>, id: &str) -> Result<()> {
    let Some(entry) = announcements
        .into_iter()
        .find(|a| a.id.eq_ignore_ascii_case(id))
    else {
        anyhow::bail!("no announcement '{id}' applies to this build");
    };
    let mut rows = vec![
        ("title".to_string(), entry.title.clone()),
        (
            "severity".to_string(),
            severity_label(entry.severity).to_string(),
        ),
        ("published".to_string(), mc::age_label(entry.published)),
    ];
    if !entry.link.is_empty() {
        rows.push(("link".to_string(), entry.link.clone()));
    }
    ui::show(View::detail(rows))?;
    // The body is markdown, printed as authored: a terminal reads it fine, and
    // rendering it would strip the structure a piped reader wants.
    ui::show(View::line(entry.body))
}
