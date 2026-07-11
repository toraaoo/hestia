//! Self-update: the released-version check and the signed installer download.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};
use crate::download::DownloadProgress;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct UpdateCheckResult {
    pub current: String,
    pub available: Option<UpdateInfo>,
}

pub struct UpdateCheck;
impl Contract for UpdateCheck {
    const CHANNEL: &'static str = "update.check";
    type Params = Empty;
    type Result = UpdateCheckResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct UpdateDownloadParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct UpdateDownloadResult {
    pub id: String,
}

pub struct UpdateDownload;
impl Contract for UpdateDownload {
    const CHANNEL: &'static str = "update.download";
    type Params = UpdateDownloadParams;
    type Result = UpdateDownloadResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: DownloadProgress,
}
impl Topic for UpdateProgressEvent {
    const TOPIC: &'static str = "update.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateDoneEvent {
    pub id: String,
    pub path: PathBuf,
    pub version: String,
}
impl Topic for UpdateDoneEvent {
    const TOPIC: &'static str = "update.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for UpdateErrorEvent {
    const TOPIC: &'static str = "update.error";
}
