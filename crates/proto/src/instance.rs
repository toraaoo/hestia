//! Minecraft instance (client) contracts: browsing (flavors/versions/resolve),
//! the stored-record management, and the launch job. Unlike servers, an
//! instance is a lightweight record at create time — its files (client jar,
//! libraries, assets) are materialised by the launch job.

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};
use crate::minecraft::{
    ConfigEntry, FlavorsResult, InstanceProfile, LoadersParams, LoadersResult, ProvisionProgress,
    ResolveParams, VersionsParams, VersionsResult,
};
use crate::process::{ProcessInfo, ProcessLogsResult};

pub struct InstanceFlavors;
impl Contract for InstanceFlavors {
    const CHANNEL: &'static str = "instance.flavors";
    type Params = Empty;
    type Result = FlavorsResult;
}

pub struct InstanceVersions;
impl Contract for InstanceVersions {
    const CHANNEL: &'static str = "instance.versions";
    type Params = VersionsParams;
    type Result = VersionsResult;
}

pub struct InstanceResolve;
impl Contract for InstanceResolve {
    const CHANNEL: &'static str = "instance.resolve";
    type Params = ResolveParams;
    type Result = InstanceProfile;
}

pub struct InstanceLoaders;
impl Contract for InstanceLoaders {
    const CHANNEL: &'static str = "instance.loaders";
    type Params = LoadersParams;
    type Result = LoadersResult;
}

/// A managed instance: the stored record plus, when launched, its live sessions.
/// An instance can run more than once concurrently (each launch is a session),
/// so this is a list, not a single process.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub flavor: String,
    pub game_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    pub java_major: i32,
    pub created_unix: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<ProcessInfo>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceCreateParams {
    /// Display name; defaults to `<flavor>-<version>` when empty.
    pub name: String,
    pub flavor: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    /// Create-time settings applied after the record is registered (memory,
    /// jvm-args).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub config: Vec<ConfigEntry>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceCreateResult {
    pub instance: InstanceInfo,
}

pub struct InstanceCreate;
impl Contract for InstanceCreate {
    const CHANNEL: &'static str = "instance.create";
    type Params = InstanceCreateParams;
    type Result = InstanceCreateResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceUpdateParams {
    /// Instance name or id.
    pub instance: String,
    /// The game version to move to (either direction; a downgrade needs
    /// `allow_downgrade`).
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    /// The caller confirms the user accepted the risk of moving to an older
    /// version (saves do not downgrade).
    pub allow_downgrade: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceUpdateResult {
    pub instance: InstanceInfo,
}

pub struct InstanceUpdate;
impl Contract for InstanceUpdate {
    const CHANNEL: &'static str = "instance.update";
    type Params = InstanceUpdateParams;
    type Result = InstanceUpdateResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceListResult {
    pub instances: Vec<InstanceInfo>,
}

pub struct InstanceList;
impl Contract for InstanceList {
    const CHANNEL: &'static str = "instance.list";
    type Params = Empty;
    type Result = InstanceListResult;
}

/// Names one managed instance by id or name (remove / stop).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceRef {
    pub instance: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceWorldsResult {
    /// Save-world folder names under the instance's `data/saves/`, sorted.
    pub worlds: Vec<String>,
}

pub struct InstanceWorlds;
impl Contract for InstanceWorlds {
    const CHANNEL: &'static str = "instance.worlds";
    type Params = InstanceRef;
    type Result = InstanceWorldsResult;
}

pub struct InstanceRemove;
impl Contract for InstanceRemove {
    const CHANNEL: &'static str = "instance.remove";
    type Params = InstanceRef;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceRenameParams {
    /// The instance to rename, by its current name or id.
    pub instance: String,
    /// The new display name; the id (directory slug) is re-derived from it.
    pub name: String,
}

pub struct InstanceRename;
impl Contract for InstanceRename {
    const CHANNEL: &'static str = "instance.rename";
    type Params = InstanceRenameParams;
    type Result = InstanceInfo;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceStopParams {
    pub instance: String,
    /// A specific session id to stop; all of the instance's sessions otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

pub struct InstanceStop;
impl Contract for InstanceStop {
    const CHANNEL: &'static str = "instance.stop";
    type Params = InstanceStopParams;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceLogsParams {
    pub instance: String,
    /// A specific session id; the newest running (else newest) session otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Return only the last `tail` lines when set; all buffered lines otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<usize>,
}

pub struct InstanceLogs;
impl Contract for InstanceLogs {
    const CHANNEL: &'static str = "instance.logs";
    type Params = InstanceLogsParams;
    type Result = ProcessLogsResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceConfigGetParams {
    pub instance: String,
    pub key: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceConfigGetResult {
    pub value: String,
}

pub struct InstanceConfigGet;
impl Contract for InstanceConfigGet {
    const CHANNEL: &'static str = "instance.config.get";
    type Params = InstanceConfigGetParams;
    type Result = InstanceConfigGetResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceConfigSetParams {
    pub instance: String,
    pub key: String,
    pub value: String,
}

pub struct InstanceConfigSet;
impl Contract for InstanceConfigSet {
    const CHANNEL: &'static str = "instance.config.set";
    type Params = InstanceConfigSetParams;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceConfigListResult {
    pub entries: Vec<ConfigEntry>,
}

pub struct InstanceConfigList;
impl Contract for InstanceConfigList {
    const CHANNEL: &'static str = "instance.config.list";
    type Params = InstanceRef;
    type Result = InstanceConfigListResult;
}

/// A named selection over the instance's installed content pool (mods,
/// resourcepacks, shaders — never datapacks). Members are pool filenames, the
/// one index field always present and unique. No profile active = every pool
/// item is mirrored.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub members: Vec<String>,
    /// Whether the profile owns a captured settings store: launches under it
    /// sync settings against `<instance>/profiles/<name>/` instead of the
    /// global `shared/` store. Uncaptured profiles inherit the global store.
    pub captured: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfileListResult {
    /// The active profile's name; empty when none is active.
    pub active: String,
    pub profiles: Vec<Profile>,
}

pub struct InstanceProfileList;
impl Contract for InstanceProfileList {
    const CHANNEL: &'static str = "instance.profile.list";
    type Params = InstanceRef;
    type Result = InstanceProfileListResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfileCreateParams {
    pub instance: String,
    pub name: String,
    /// Start with every selectable pool item as a member; off creates empty.
    pub seed_from_pool: bool,
}

pub struct InstanceProfileCreate;
impl Contract for InstanceProfileCreate {
    const CHANNEL: &'static str = "instance.profile.create";
    type Params = InstanceProfileCreateParams;
    type Result = Profile;
}

/// Names one profile of one instance (remove / use).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfileRef {
    pub instance: String,
    pub name: String,
}

/// Removing the active profile clears the active selection.
pub struct InstanceProfileRemove;
impl Contract for InstanceProfileRemove {
    const CHANNEL: &'static str = "instance.profile.remove";
    type Params = InstanceProfileRef;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfileRenameParams {
    pub instance: String,
    pub name: String,
    pub new_name: String,
}

pub struct InstanceProfileRename;
impl Contract for InstanceProfileRename {
    const CHANNEL: &'static str = "instance.profile.rename";
    type Params = InstanceProfileRenameParams;
    type Result = Profile;
}

/// An empty `name` clears the active profile.
pub struct InstanceProfileUse;
impl Contract for InstanceProfileUse {
    const CHANNEL: &'static str = "instance.profile.use";
    type Params = InstanceProfileRef;
    type Result = Empty;
}

/// Capture the profile's own settings store, snapshotted from the global
/// `shared/` store; from then on launches under the profile sync against it.
/// Divergence after capture is by design.
pub struct InstanceProfileCapture;
impl Contract for InstanceProfileCapture {
    const CHANNEL: &'static str = "instance.profile.capture";
    type Params = InstanceProfileRef;
    type Result = Empty;
}

/// Delete the profile's captured store; the profile inherits the global
/// `shared/` store again from the next launch.
pub struct InstanceProfileRelease;
impl Contract for InstanceProfileRelease {
    const CHANNEL: &'static str = "instance.profile.release";
    type Params = InstanceProfileRef;
    type Result = Empty;
}

/// `add`/`remove` are pool references (project id, slug, filename, or title),
/// resolved server-side; one that matches nothing — or only a datapack — is a
/// `bad_request`.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfileEditParams {
    pub instance: String,
    pub name: String,
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

pub struct InstanceProfileEdit;
impl Contract for InstanceProfileEdit {
    const CHANNEL: &'static str = "instance.profile.edit";
    type Params = InstanceProfileEditParams;
    type Result = Profile;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceLaunchParams {
    pub instance: String,
    /// Account name or uuid; empty picks the sole signed-in account.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub account: String,
    /// Client-supplied job id; empty asks the daemon to allocate one.
    pub id: String,
    /// Launch another session even when one is already running. Off by default:
    /// a running instance is refused unless the caller opts into concurrency.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub new_session: bool,
    /// A profile override for this launch only: empty uses the active profile,
    /// the literal `none` launches with no profile. `none` (and empty) are
    /// therefore reserved as profile names.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceLaunchResult {
    pub id: String,
}

pub struct InstanceLaunch;
impl Contract for InstanceLaunch {
    const CHANNEL: &'static str = "instance.launch";
    type Params = InstanceLaunchParams;
    type Result = InstanceLaunchResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceLaunchProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: ProvisionProgress,
}
impl Topic for InstanceLaunchProgressEvent {
    const TOPIC: &'static str = "instance.launch.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceLaunchDoneEvent {
    pub id: String,
    pub process_id: String,
    pub pid: u32,
}
impl Topic for InstanceLaunchDoneEvent {
    const TOPIC: &'static str = "instance.launch.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstanceLaunchErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for InstanceLaunchErrorEvent {
    const TOPIC: &'static str = "instance.launch.error";
}
