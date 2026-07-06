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
    Java,
    Server,
    Client,
    Libraries,
    Assets,
}

/// Progress for a provisioning job. `current`/`total` are bytes for a
/// single-artifact phase and completed/total counts for `Libraries`/`Assets`;
/// a phase with unknown extent reports `0/0`.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProvisionProgress {
    pub phase: ProvisionPhase,
    pub current: u64,
    pub total: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
}
