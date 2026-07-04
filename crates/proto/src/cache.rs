use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty};
use crate::download::Checksum;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheEntry {
    #[serde(flatten)]
    pub checksum: Checksum,
    #[serde(default)]
    pub size: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct CacheUsage {
    pub entries: u64,
    pub bytes: u64,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct CacheInfoResult {
    pub path: PathBuf,
    #[serde(flatten)]
    pub usage: CacheUsage,
}

pub struct CacheInfo;
impl Contract for CacheInfo {
    const CHANNEL: &'static str = "cache.info";
    type Params = Empty;
    type Result = CacheInfoResult;
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct CacheListResult {
    pub entries: Vec<CacheEntry>,
}

pub struct CacheList;
impl Contract for CacheList {
    const CHANNEL: &'static str = "cache.list";
    type Params = Empty;
    type Result = CacheListResult;
}

pub struct CacheClear;
impl Contract for CacheClear {
    const CHANNEL: &'static str = "cache.clear";
    type Params = Empty;
    type Result = CacheUsage;
}
