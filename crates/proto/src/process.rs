use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};

/// What the supervisor does when a launched process exits.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Leave the process dead once it exits (the default).
    #[default]
    Never,
    /// Re-spawn it if it exits non-zero, up to a bounded number of attempts.
    OnFailure,
}

/// Where a supervised process's output ends up.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    /// The supervisor captures stdout+stderr into a file it owns.
    #[default]
    Capture,
    /// The process writes its own log; the supervisor tails this file instead.
    File(PathBuf),
}

/// A request to launch a process under the daemon's supervisor.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessSpec {
    /// Client-supplied id; empty asks the daemon to allocate one.
    pub id: String,
    pub program: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub restart: RestartPolicy,
    pub log: LogSource,
}

/// Where a supervised process is in its lifecycle.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessState {
    #[default]
    Running,
    /// Exited on its own (cleanly or not — see `exit_code`).
    Exited,
    /// Terminated by a `process.stop`.
    Killed,
}

/// A snapshot of one tracked process.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessInfo {
    pub id: String,
    pub pid: u32,
    pub program: String,
    pub args: Vec<String>,
    pub state: ProcessState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub started_unix: i64,
}

/// Which stream a captured log line came from.
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    #[default]
    Stdout,
    Stderr,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessLogLine {
    pub stream: LogStream,
    pub line: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessStartResult {
    pub id: String,
    pub pid: u32,
}

pub struct ProcessStart;
impl Contract for ProcessStart {
    const CHANNEL: &'static str = "process.start";
    type Params = ProcessSpec;
    type Result = ProcessStartResult;
}

/// Names a single tracked process by id (stop / status / logs share it).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessRef {
    pub id: String,
}

pub struct ProcessStop;
impl Contract for ProcessStop {
    const CHANNEL: &'static str = "process.stop";
    type Params = ProcessRef;
    type Result = Empty;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessListResult {
    pub processes: Vec<ProcessInfo>,
}

pub struct ProcessList;
impl Contract for ProcessList {
    const CHANNEL: &'static str = "process.list";
    type Params = Empty;
    type Result = ProcessListResult;
}

pub struct ProcessStatus;
impl Contract for ProcessStatus {
    const CHANNEL: &'static str = "process.status";
    type Params = ProcessRef;
    type Result = ProcessInfo;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessLogsParams {
    pub id: String,
    /// Return only the last `tail` lines when set; all buffered lines otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail: Option<usize>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessLogsResult {
    pub lines: Vec<ProcessLogLine>,
}

pub struct ProcessLogs;
impl Contract for ProcessLogs {
    const CHANNEL: &'static str = "process.logs";
    type Params = ProcessLogsParams;
    type Result = ProcessLogsResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessStartedEvent {
    pub id: String,
    pub pid: u32,
}
impl Topic for ProcessStartedEvent {
    const TOPIC: &'static str = "process.started";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessOutputEvent {
    pub id: String,
    #[serde(flatten)]
    pub line: ProcessLogLine,
}
impl Topic for ProcessOutputEvent {
    const TOPIC: &'static str = "process.output";
}

/// One running process's resource sample; `cpu_pct` is 100 per full core.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProcessMetrics {
    pub id: String,
    pub cpu_pct: f32,
    pub mem_bytes: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessMetricsEvent {
    pub samples: Vec<ProcessMetrics>,
}
impl Topic for ProcessMetricsEvent {
    const TOPIC: &'static str = "process.metrics";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProcessExitEvent {
    pub id: String,
    pub state: ProcessState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub success: bool,
}
impl Topic for ProcessExitEvent {
    const TOPIC: &'static str = "process.exit";
}
