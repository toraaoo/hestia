//! The provider seams. A flavor implements the trait for its domain (server or
//! instance), listing the game versions it supports and resolving a request into
//! a full launch profile. The `Minecraft` aggregate holds a boxed registry of
//! each — adding a flavor is a new impl plus one line in `Minecraft::new`.

use anyhow::Result;
use async_trait::async_trait;
use proto::minecraft::{GameVersion, InstanceProfile, ServerProfile};

/// A resolution request: a game version and, for modloaders, an optional pinned
/// loader version (the newest stable loader is chosen when absent).
pub struct ResolveRequest {
    pub version: String,
    pub loader_version: Option<String>,
}

#[async_trait]
pub trait ServerProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn versions(&self) -> Result<Vec<GameVersion>>;
    async fn resolve(&self, request: &ResolveRequest) -> Result<ServerProfile>;
}

#[async_trait]
pub trait InstanceProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn versions(&self) -> Result<Vec<GameVersion>>;
    async fn resolve(&self, request: &ResolveRequest) -> Result<InstanceProfile>;
}
