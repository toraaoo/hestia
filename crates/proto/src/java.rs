use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaRelease {
    pub major: i32,
    pub lts: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaRuntime {
    pub vendor: String,
    pub major: i32,
    pub release_name: String,
    pub home: PathBuf,
    pub executable: PathBuf,
    /// Whether an existing server or instance launches with this major.
    pub in_use: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JavaInstallPhase {
    Resolving,
    Downloading,
    Extracting,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaInstallProgress {
    pub phase: JavaInstallPhase,
    #[serde(default)]
    pub current: u64,
    #[serde(default)]
    pub total: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaReleasesResult {
    pub releases: Vec<JavaRelease>,
}

pub struct JavaReleases;
impl Contract for JavaReleases {
    const CHANNEL: &'static str = "java.releases";
    type Params = Empty;
    type Result = JavaReleasesResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaListResult {
    pub runtimes: Vec<JavaRuntime>,
}

pub struct JavaList;
impl Contract for JavaList {
    const CHANNEL: &'static str = "java.list";
    type Params = Empty;
    type Result = JavaListResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaInstallParams {
    pub major: i32,
    pub id: String,
    pub force: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaInstallResult {
    pub id: String,
}

pub struct JavaInstall;
impl Contract for JavaInstall {
    const CHANNEL: &'static str = "java.install";
    type Params = JavaInstallParams;
    type Result = JavaInstallResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct JavaUninstallParams {
    pub major: i32,
}

pub struct JavaUninstall;
impl Contract for JavaUninstall {
    const CHANNEL: &'static str = "java.uninstall";
    type Params = JavaUninstallParams;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaInstallProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: JavaInstallProgress,
}
impl Topic for JavaInstallProgressEvent {
    const TOPIC: &'static str = "java.install.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaInstallDoneEvent {
    pub id: String,
    pub runtime: JavaRuntime,
    #[serde(default)]
    pub already_installed: bool,
}
impl Topic for JavaInstallDoneEvent {
    const TOPIC: &'static str = "java.install.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaInstallErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for JavaInstallErrorEvent {
    const TOPIC: &'static str = "java.install.error";
}
