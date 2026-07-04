use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct DaemonStatusResult {
    pub pid: i64,
    pub version: String,
    pub uptime_seconds: i64,
    pub home: PathBuf,
    pub log: PathBuf,
}

pub struct DaemonStatus;
impl Contract for DaemonStatus {
    const CHANNEL: &'static str = "daemon.status";
    type Params = Empty;
    type Result = DaemonStatusResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct DaemonStopResult {
    pub stopping: bool,
}

pub struct DaemonStop;
impl Contract for DaemonStop {
    const CHANNEL: &'static str = "daemon.stop";
    type Params = Empty;
    type Result = DaemonStopResult;
}
