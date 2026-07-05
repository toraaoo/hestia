//! Minecraft server contracts: browsing (flavors/versions/resolve), the
//! provision-at-create job, and lifecycle over the daemon's process supervisor.
//! A server is provisioned fully at create time — profile resolved, the Java
//! runtime ensured, the server jar downloaded, the EULA recorded — so `start`
//! is an immediate spawn.

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};
use crate::minecraft::{
    FlavorsResult, ProvisionProgress, ResolveParams, ServerProfile, VersionsParams, VersionsResult,
};
use crate::process::{ProcessInfo, ProcessLogsResult};

pub struct ServerFlavors;
impl Contract for ServerFlavors {
    const CHANNEL: &'static str = "server.flavors";
    type Params = Empty;
    type Result = FlavorsResult;
}

pub struct ServerVersions;
impl Contract for ServerVersions {
    const CHANNEL: &'static str = "server.versions";
    type Params = VersionsParams;
    type Result = VersionsResult;
}

pub struct ServerResolve;
impl Contract for ServerResolve {
    const CHANNEL: &'static str = "server.resolve";
    type Params = ResolveParams;
    type Result = ServerProfile;
}

/// A managed server: the stored record plus, when it has been started, the
/// supervised process snapshot.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub flavor: String,
    pub game_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    pub java_major: i32,
    pub created_unix: i64,
    /// False while the create job is still provisioning files.
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<ProcessInfo>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerCreateParams {
    /// Display name; defaults to `<flavor>-<version>` when empty.
    pub name: String,
    pub flavor: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    /// The caller confirms the user accepted the Minecraft EULA.
    pub eula: bool,
    /// Client-supplied job id; empty asks the daemon to allocate one.
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerCreateResult {
    pub id: String,
}

pub struct ServerCreate;
impl Contract for ServerCreate {
    const CHANNEL: &'static str = "server.create";
    type Params = ServerCreateParams;
    type Result = ServerCreateResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerListResult {
    pub servers: Vec<ServerInfo>,
}

pub struct ServerList;
impl Contract for ServerList {
    const CHANNEL: &'static str = "server.list";
    type Params = Empty;
    type Result = ServerListResult;
}

/// Names one managed server by id or name (remove / start / stop / status).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerRef {
    pub server: String,
}

pub struct ServerRemove;
impl Contract for ServerRemove {
    const CHANNEL: &'static str = "server.remove";
    type Params = ServerRef;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerStartResult {
    pub process_id: String,
    pub pid: u32,
}

pub struct ServerStart;
impl Contract for ServerStart {
    const CHANNEL: &'static str = "server.start";
    type Params = ServerRef;
    type Result = ServerStartResult;
}

pub struct ServerStop;
impl Contract for ServerStop {
    const CHANNEL: &'static str = "server.stop";
    type Params = ServerRef;
    type Result = Empty;
}

pub struct ServerStatus;
impl Contract for ServerStatus {
    const CHANNEL: &'static str = "server.status";
    type Params = ServerRef;
    type Result = ServerInfo;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerLogsParams {
    pub server: String,
    /// Return only the last `tail` lines when set; all buffered lines otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<usize>,
}

pub struct ServerLogs;
impl Contract for ServerLogs {
    const CHANNEL: &'static str = "server.logs";
    type Params = ServerLogsParams;
    type Result = ProcessLogsResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerCreateProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: ProvisionProgress,
}
impl Topic for ServerCreateProgressEvent {
    const TOPIC: &'static str = "server.create.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerCreateDoneEvent {
    pub id: String,
    pub server: ServerInfo,
}
impl Topic for ServerCreateDoneEvent {
    const TOPIC: &'static str = "server.create.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerCreateErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for ServerCreateErrorEvent {
    const TOPIC: &'static str = "server.create.error";
}
