use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Topic};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Sha1,
    Sha256,
}

impl HashAlgorithm {
    /// Length of the algorithm's digest in hex characters (40 / 64).
    pub fn hex_digest_length(self) -> usize {
        match self {
            HashAlgorithm::Sha1 => 40,
            HashAlgorithm::Sha256 => 64,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Sha256 => "sha256",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    pub algorithm: HashAlgorithm,
    #[serde(default)]
    pub hex: String,
}

impl Checksum {
    /// Well-formed when `hex` is exactly the algorithm's digest length and holds
    /// only hex characters — one definition every caller validates against.
    pub fn is_valid(&self) -> bool {
        self.hex.len() == self.algorithm.hex_digest_length()
            && self.hex.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSpec {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "dest", default)]
    pub destination: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct DownloadStartResult {
    pub id: String,
}

pub struct DownloadStart;
impl Contract for DownloadStart {
    const CHANNEL: &'static str = "download.start";
    type Params = DownloadSpec;
    type Result = DownloadStartResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: DownloadProgress,
}
impl Topic for DownloadProgressEvent {
    const TOPIC: &'static str = "download.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadDoneEvent {
    pub id: String,
    pub path: PathBuf,
}
impl Topic for DownloadDoneEvent {
    const TOPIC: &'static str = "download.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for DownloadErrorEvent {
    const TOPIC: &'static str = "download.error";
}
