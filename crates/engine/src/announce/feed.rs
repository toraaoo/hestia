//! The published feed document and the rule deciding which of its entries this
//! build should see.
//!
//! The document is *ours*, not an upstream API, so it is camelCase like every
//! other record here. Its shape is deliberately wider than `proto`'s
//! [`Announcement`]: the targeting fields exist only to be applied, and are
//! dropped at this boundary so a front-end never sees them.

use proto::announce::{Announcement, Severity};
use serde::Deserialize;

use crate::version;

/// The signed wrapper the feed is published as. The payload travels as *text*,
/// not a nested object, because the signature covers exact bytes — reserializing
/// a parsed object would change them. Verify, then parse.
#[derive(Deserialize)]
pub struct Envelope {
    pub signature: String,
    pub payload: String,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Feed {
    pub version: u32,
    pub entries: Vec<Entry>,
}

/// One authored announcement, before targeting is applied.
///
/// `PartialEq` is what a poll compares: an entry is "the same" only when every
/// authored field matches, so an edit to a published body is a change even
/// though the id did not move.
#[derive(Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub body: String,
    pub link: String,
    pub published: i64,
    /// Unix seconds after which the entry stops applying; 0 never expires.
    pub expires: i64,
    /// `std::env::consts::OS` values. Empty applies everywhere.
    pub platforms: Vec<String>,
    /// `common::app::CHANNEL` values. Empty applies to every channel.
    pub channels: Vec<String>,
    pub min_version: String,
    pub max_version: String,
}

/// The feed schema this build understands. A document declaring a higher
/// version is refused whole rather than half-read: a future schema may change
/// what a field *means*, and silently applying old rules to new data is how a
/// targeted notice reaches the wrong builds.
pub const SCHEMA_VERSION: u32 = 1;

impl Entry {
    /// Whether this entry applies to the running build. Every targeting field
    /// is a filter that an empty value opens — MultiMC's rule, and the one that
    /// makes an untargeted entry the natural default.
    pub fn applies(&self, ctx: &Context) -> bool {
        if self.id.trim().is_empty() {
            return false;
        }
        if self.expires != 0 && ctx.now >= self.expires {
            return false;
        }
        if !self.platforms.is_empty() && !self.platforms.iter().any(|p| p == ctx.platform) {
            return false;
        }
        if !self.channels.is_empty() && !self.channels.iter().any(|c| c == ctx.channel) {
            return false;
        }
        version::in_range(ctx.version, &self.min_version, &self.max_version)
    }

    pub fn into_announcement(self, dismissed: bool) -> Announcement {
        Announcement {
            id: self.id,
            severity: self.severity,
            title: self.title,
            body: self.body,
            link: self.link,
            published: self.published,
            dismissed,
        }
    }
}

/// What an entry is filtered against — the running build, passed in rather than
/// read from globals so the rules are testable.
pub struct Context<'a> {
    pub platform: &'a str,
    pub channel: &'a str,
    pub version: &'a str,
    pub now: i64,
}

impl Context<'_> {
    pub fn current(now: i64) -> Context<'static> {
        Context {
            platform: std::env::consts::OS,
            channel: common::app::CHANNEL,
            version: common::app::VERSION,
            now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, Entry};

    fn ctx() -> Context<'static> {
        Context {
            platform: "linux",
            channel: "dev",
            version: "0.0.2",
            now: 1_000,
        }
    }

    fn entry() -> Entry {
        Entry {
            id: "an-id".into(),
            ..Default::default()
        }
    }

    #[test]
    fn an_untargeted_entry_applies_to_everyone() {
        assert!(entry().applies(&ctx()));
    }

    #[test]
    fn an_entry_with_no_id_is_refused() {
        // The id is the dismissal key; without one it could never be dismissed
        // and would reappear on every poll.
        let e = Entry {
            id: "  ".into(),
            ..Default::default()
        };
        assert!(!e.applies(&ctx()));
    }

    #[test]
    fn platform_and_channel_filter_when_named() {
        let windows = Entry {
            platforms: vec!["windows".into()],
            ..entry()
        };
        assert!(!windows.applies(&ctx()));

        let both = Entry {
            platforms: vec!["windows".into(), "linux".into()],
            ..entry()
        };
        assert!(both.applies(&ctx()));

        let stable = Entry {
            channels: vec!["stable".into()],
            ..entry()
        };
        assert!(!stable.applies(&ctx()));
    }

    #[test]
    fn expiry_is_exclusive_of_the_moment_it_passes() {
        let expired = Entry {
            expires: 1_000,
            ..entry()
        };
        assert!(!expired.applies(&ctx()));

        let live = Entry {
            expires: 1_001,
            ..entry()
        };
        assert!(live.applies(&ctx()));

        let never = Entry {
            expires: 0,
            ..entry()
        };
        assert!(never.applies(&ctx()));
    }

    #[test]
    fn a_version_range_selects_affected_builds() {
        let affected = Entry {
            min_version: "0.0.1".into(),
            max_version: "0.0.3".into(),
            ..entry()
        };
        assert!(affected.applies(&ctx()));

        let older_only = Entry {
            max_version: "0.0.1".into(),
            ..entry()
        };
        assert!(!older_only.applies(&ctx()));

        let newer_only = Entry {
            min_version: "0.0.3".into(),
            ..entry()
        };
        assert!(!newer_only.applies(&ctx()));
    }

    #[test]
    fn a_malformed_range_reaches_nobody() {
        // Fails closed: a typo in a published bound must not broadcast a
        // targeted notice to every build.
        let broken = Entry {
            min_version: "one.two".into(),
            ..entry()
        };
        assert!(!broken.applies(&ctx()));
    }
}
