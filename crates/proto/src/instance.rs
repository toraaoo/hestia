//! Minecraft instance (client) contracts: browsing (flavors/versions/resolve),
//! the stored-record management, and the launch job. Unlike servers, an
//! instance is a lightweight record at create time — its files (client jar,
//! libraries, assets) are materialised by the launch job.

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};
use crate::minecraft::{
    FlavorsResult, InstanceProfile, ProvisionProgress, ResolveParams, VersionsParams,
    VersionsResult,
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

/// A managed instance: the stored record plus, when launched, the supervised
/// process snapshot.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessInfo>,
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

pub struct InstanceRemove;
impl Contract for InstanceRemove {
    const CHANNEL: &'static str = "instance.remove";
    type Params = InstanceRef;
    type Result = Empty;
}

pub struct InstanceStop;
impl Contract for InstanceStop {
    const CHANNEL: &'static str = "instance.stop";
    type Params = InstanceRef;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceLogsParams {
    pub instance: String,
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
pub struct InstanceLaunchParams {
    pub instance: String,
    /// Account name or uuid; empty picks the sole signed-in account.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub account: String,
    /// Client-supplied job id; empty asks the daemon to allocate one.
    pub id: String,
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
