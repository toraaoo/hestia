//! Announcements: the news and notices the launcher fetches from its published
//! feed. An entry is authored as markdown in the repo and compiled into one
//! signed document; the engine verifies it, drops what does not apply to this
//! build, and serves what is left.
//!
//! Targeting (platform, release channel, version range, expiry) is deliberately
//! **not** on the wire: the engine has already applied it, so a front-end
//! renders what it is given rather than keeping a second copy of the rule.

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};

/// How loudly an announcement should be presented. `Critical` is for things a
/// user must act on (a data-loss bug, a compromised release); `Info` is news.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    #[default]
    Info,
    Warning,
    Critical,
}

/// One announcement that applies to this build. `body` is markdown; a
/// front-end must render it as untrusted input.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct Announcement {
    /// Stable and permanent — the dismissal key. Reusing an id silently hides
    /// a new announcement from everyone who dismissed the old one.
    pub id: String,
    pub severity: Severity,
    pub title: String,
    /// Markdown. Rendered as untrusted input on every front-end.
    pub body: String,
    /// A "read more" URL; empty when the entry has none.
    pub link: String,
    /// Publication time, unix seconds.
    pub published: i64,
    pub dismissed: bool,
}

/// Every announcement applying to this build, newest first — dismissed ones
/// included, flagged rather than dropped, so a front-end can offer both an
/// unread badge and a full history.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct AnnounceListResult {
    pub announcements: Vec<Announcement>,
    /// When the feed was last fetched successfully, unix seconds; 0 = never.
    /// A front-end reports staleness from this rather than being told the
    /// last fetch failed — an unreachable feed is a state, not an error.
    pub fetched: i64,
}

pub struct AnnounceList;
impl Contract for AnnounceList {
    const CHANNEL: &'static str = "announce.list";
    type Params = Empty;
    type Result = AnnounceListResult;
}

/// Mark announcements read. Ids are named explicitly — "everything currently
/// shown" is the caller's list, not a meaning the daemon infers from an empty
/// set — and an id that no longer applies is accepted and remembered, so a
/// re-published entry stays dismissed.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct AnnounceDismissParams {
    pub ids: Vec<String>,
}

pub struct AnnounceDismiss;
impl Contract for AnnounceDismiss {
    const CHANNEL: &'static str = "announce.dismiss";
    type Params = AnnounceDismissParams;
    type Result = AnnounceListResult;
}

/// Fetch the feed now rather than waiting for the poll. Answers from cache if
/// the fetch fails, so a refresh on a dead network still returns a list.
pub struct AnnounceRefresh;
impl Contract for AnnounceRefresh {
    const CHANNEL: &'static str = "announce.refresh";
    type Params = Empty;
    type Result = AnnounceListResult;
}

/// Pushed when a poll changes what applies to this build, so a front-end
/// updates its badge without holding a query open.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(rename_all = "camelCase")]
pub struct AnnounceChangedEvent {
    pub unread: u32,
}
impl Topic for AnnounceChangedEvent {
    const TOPIC: &'static str = "announce.changed";
}
