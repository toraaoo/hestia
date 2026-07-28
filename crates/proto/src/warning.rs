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
/// shared store. A folder holding only the instance's own files is adopted
/// automatically, so both of these are cases where a move would have destroyed
/// something.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum NotSharedReason {
    /// The store already holds files by the same names, so moving the folder's
    /// own into it would overwrite them.
    Collides,
    /// The folder is a symlink the user made, pointing somewhere that is not a
    /// hestia store. Only hestia's own links are ever touched.
    ForeignLink,
}

impl fmt::Display for NotSharedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NotSharedReason::Collides => "files of the same name are already shared",
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
    /// `count` alongside `paths` so the headline is one interpolation rather
    /// than a joined list a front-end would have to shorten itself.
    ModpackOverridesKept { count: u32, paths: Vec<String> },
    /// The pack declared client-only mods as server-compatible, so they were
    /// held back — a correction over the pack's own `env`, which packs get
    /// wrong routinely. Named individually: which mod was dropped is exactly
    /// what someone debugging a pack needs, and the list is short.
    ModpackFilesExcluded { count: u32, files: Vec<String> },
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
                target,
                reason: NotSharedReason::Collides,
                ..
            } => format!(
                "rename or delete the clashing files under `data/{target}`, then launch again to \
                 share it"
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
            ModpackFilesExcluded { .. } => {
                "they would break a server; `config set modpack.force-include-files <name>` \
                 installs one anyway, and `modpack.default-excludes false` trusts the pack"
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
            ModpackOverridesKept { count, .. } => write!(
                f,
                "{count} file(s) you had edited were kept instead of the modpack's"
            ),
            ModpackFilesExcluded { count, .. } => write!(
                f,
                "{count} client-only mod(s) the pack called server-compatible were not installed"
            ),
        }
    }
}
