//! Minecraft provider vocabulary shared by both sides of the socket. A *flavor*
//! is a distribution (vanilla, fabric, …); a provider lists the game *versions*
//! it supports and *resolves* a request into a launch profile — the full
//! descriptor the launch pipeline consumes. Servers and instances (clients)
//! share this vocabulary but resolve to different profiles; their contracts live
//! in the `server` and `instance` modules.

use serde::{Deserialize, Serialize};

use crate::download::Checksum;

/// A distribution offered by a domain: the first level of the `available`
/// selector (`vanilla`, `fabric`, …).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Flavor {
    pub id: String,
    pub name: String,
}

/// One key/value setting, shared by the server and instance `config` channels
/// and their create-time settings list.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VersionKind {
    #[default]
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

/// A game version a flavor can target.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct GameVersion {
    pub id: String,
    pub kind: VersionKind,
    pub stable: bool,
}

/// A single downloadable file, the shared shape for every artifact in a profile.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Artifact {
    pub url: String,
    pub filename: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,
}

/// A classpath dependency, resolved to its download and its path under the
/// libraries root (Maven layout).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Library {
    pub name: String,
    pub path: String,
    pub artifact: Artifact,
}

/// The asset index a client version pins (`assetIndex` in a vanilla profile).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct AssetIndex {
    pub id: String,
    pub artifact: Artifact,
    pub total_size: u64,
}

/// The resolved launch profile for a Minecraft *server*.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerProfile {
    pub flavor: String,
    pub game_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    pub primary: Artifact,
    pub libraries: Vec<Library>,
    pub java_major: i32,
    pub main_class: String,
}

/// The resolved launch profile for a Minecraft *client* (instance).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfile {
    pub flavor: String,
    pub game_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    pub client: Artifact,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndex,
    pub java_major: i32,
    pub main_class: String,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
}

/// Whether moving `from` → `to` is a downgrade, judged by their positions in a
/// provider's newest-first version list — the catalogue is the ordering ground
/// truth, so no version-string parsing can drift from upstream. `None` when
/// either version is not listed.
pub fn downgrade_between(versions: &[GameVersion], from: &str, to: &str) -> Option<bool> {
    let position = |id: &str| versions.iter().position(|v| v.id == id);
    Some(position(to)? > position(from)?)
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct FlavorsResult {
    pub flavors: Vec<Flavor>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct VersionsParams {
    pub flavor: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct VersionsResult {
    pub versions: Vec<GameVersion>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct LoadersParams {
    pub flavor: String,
    pub version: String,
}

/// Loader builds newest-first; empty for a flavor with no pickable loader
/// version (vanilla, and loaders that pin one build per game version).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct LoadersResult {
    pub loaders: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ResolveParams {
    pub flavor: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
}

/// Where a provisioning job (server create, instance launch preparation) is.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionPhase {
    #[default]
    Resolving,
    Backup,
    Java,
    Server,
    Client,
    Libraries,
    Assets,
    Content,
}

/// Progress for a provisioning job. `current`/`total` are bytes for a
/// single-artifact phase and completed/total counts for `Libraries`/`Assets`;
/// a phase with unknown extent reports `0/0`. A phase made of several units
/// (a content batch installing many projects) also carries which unit this
/// progress belongs to: `item` (1-based) of `items` — `items` may grow while
/// the job runs, as dependency resolution discovers more work. Both are zero
/// for a single-unit phase.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProvisionProgress {
    pub phase: ProvisionPhase,
    pub current: u64,
    pub total: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
    #[serde(skip_serializing_if = "u64_is_zero")]
    pub item: u64,
    #[serde(skip_serializing_if = "u64_is_zero")]
    pub items: u64,
}

fn u64_is_zero(value: &u64) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions(ids: &[&str]) -> Vec<GameVersion> {
        ids.iter()
            .map(|id| GameVersion {
                id: id.to_string(),
                ..GameVersion::default()
            })
            .collect()
    }

    #[test]
    fn downgrade_follows_list_order() {
        let list = versions(&["1.21.1", "1.21", "1.20.4"]);
        assert_eq!(downgrade_between(&list, "1.20.4", "1.21.1"), Some(false));
        assert_eq!(downgrade_between(&list, "1.21.1", "1.20.4"), Some(true));
        assert_eq!(downgrade_between(&list, "1.21", "1.21"), Some(false));
    }

    #[test]
    fn unknown_versions_are_undecidable() {
        let list = versions(&["1.21.1", "1.21"]);
        assert_eq!(downgrade_between(&list, "1.8.9", "1.21"), None);
        assert_eq!(downgrade_between(&list, "1.21", "nope"), None);
    }
}
