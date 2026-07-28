//! The canonical daemon warning — a degraded outcome of an operation that
//! nonetheless succeeded. Structured exactly like [`crate::error::ErrorInfo`]:
//! nobody authors prose at a call site, the English text (`Display`) is a
//! projection of the variant, and a front-end renders its own localized string
//! from the tag + typed fields.
//!
//! A warning exists because the alternative is a daemon log line, which the
//! person who ran the operation never sees — so an operation that half-worked
//! reported unqualified success. Anything the user would want to know about a
//! result belongs here, on the result.
//!
//! Every variant carries a [`WarningInfo::hint`] beside its headline: a
//! degraded outcome the user cannot act on is just noise, so the remediation is
//! part of the type rather than something each front-end invents.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Why a folder target stayed instance-local instead of being linked into the
/// shared store — the empty-or-linked guard's two refusals.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum NotSharedReason {
    /// The folder is a real directory with contents; linking would have merged
    /// or replaced them, so it was left alone.
    HasContents,
    /// The folder is a symlink the user made, pointing somewhere that is not a
    /// hestia store. Only hestia's own links are ever touched.
    ForeignLink,
}

impl fmt::Display for NotSharedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NotSharedReason::HasContents => "the folder already has contents",
            NotSharedReason::ForeignLink => "the folder is a link you made",
        })
    }
}

/// One degraded outcome. The `kind` tag is the wire discriminant; front-ends
/// switch on it exhaustively.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WarningInfo {
    /// The schema-generation run produced nothing, so this server's property
    /// keys cannot be validated — every unmanaged key will be accepted.
    PropertiesSchemaMissing { name: String },
    /// A shared folder target was left instance-local, so the game runs against
    /// that instance's own copy rather than the shared store.
    SyncTargetNotShared {
        instance: String,
        target: String,
        reason: NotSharedReason,
    },
    /// A sync target could not be reconciled at all. `detail` is operational
    /// English, shown as secondary text.
    SyncTargetSkipped { target: String, detail: String },
    /// Game-directory files the pack owns were left as the user edited them, so
    /// this entry is not running the pack's own configuration for them.
    ModpackOverridesKept { paths: Vec<String> },
    /// The pack named files of a kind this entry's flavor cannot load, so they
    /// were not installed — a client-shaped pack put on a server, typically.
    ModpackFilesNotAccepted { count: u32, flavor: String },
}

impl WarningInfo {
    /// What the user can do about it — the whole point of surfacing a warning
    /// rather than logging it. Rendered as secondary text under the headline;
    /// a front-end localizes it from the same tag + fields.
    pub fn hint(&self) -> String {
        use WarningInfo::*;
        match self {
            PropertiesSchemaMissing { name } => format!(
                "any key is accepted until it can be derived again, so check spelling yourself; \
                 `hestia server {name} update <version>` re-derives it"
            ),
            SyncTargetNotShared {
                instance,
                target,
                reason: NotSharedReason::HasContents,
            } => format!(
                "`hestia instance {instance} sync adopt {target}` moves those contents into the \
                 shared store and links the folder"
            ),
            SyncTargetNotShared { target, .. } => format!(
                "remove or repoint the link at `data/{target}`, then launch again to share it"
            ),
            SyncTargetSkipped { target, .. } => {
                format!("check permissions on `data/{target}`, then launch again")
            }
            ModpackOverridesKept { .. } => {
                "delete a file under `data/` to take the pack's version of it at the next update"
                    .to_string()
            }
            ModpackFilesNotAccepted { .. } => {
                "nothing to do — the pack ships them for the other side; install the pack on an \
                 instance to get them"
                    .to_string()
            }
        }
    }
}

impl fmt::Display for WarningInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use WarningInfo::*;
        match self {
            PropertiesSchemaMissing { name } => write!(
                f,
                "'{name}' has no property schema: its server.properties keys cannot be validated"
            ),
            SyncTargetNotShared { target, reason, .. } => write!(
                f,
                "'{target}' is not shared with your other instances: {reason}"
            ),
            SyncTargetSkipped { target, detail } => {
                write!(f, "'{target}' could not be synced: {detail}")
            }
            ModpackOverridesKept { paths } => write!(
                f,
                "{} file(s) you had edited were kept instead of the modpack's",
                paths.len()
            ),
            ModpackFilesNotAccepted { count, flavor } => write!(
                f,
                "{count} of the modpack's file(s) are not loaded by a {flavor} entry and were skipped"
            ),
        }
    }
}
