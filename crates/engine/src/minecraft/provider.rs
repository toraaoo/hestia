//! The provider seams. A flavor implements the trait for its domain (server or
//! instance), listing the game versions it supports and resolving a request into
//! a full launch profile. The `Minecraft` aggregate holds a boxed registry of
//! each — adding a flavor is a new impl plus one line in `Minecraft::new`.

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use proto::content::ContentKind;
use proto::minecraft::{GameVersion, InstanceProfile, ServerProfile};

use crate::cache::Cache;
use crate::minecraft::materialize::OnProgress;

/// A resolution request: a game version and, for modloaders, an optional pinned
/// loader version (the newest stable loader is chosen when absent).
pub struct ResolveRequest {
    pub version: String,
    pub loader_version: Option<String>,
}

/// What a flavor needs to build whatever its profile cannot simply name.
///
/// Most flavors need nothing: a profile is a list of downloads, and the
/// materialize pass fetches it. NeoForge is the exception — the jar its loader
/// runs does not exist anywhere to download and is produced locally from the
/// vanilla one, so the flavor gets a hook rather than the launch flows growing a
/// branch on a flavor name.
pub struct InstallRequest<'a> {
    pub game_version: &'a str,
    pub loader_version: Option<&'a str>,
    /// The root the install writes under — `meta/` for a client (whose
    /// `libraries/` is the shared root) and the server's own data directory.
    pub root: &'a Path,
    /// The vanilla jar for this side, already materialized.
    pub minecraft_jar: &'a Path,
    pub java: &'a Path,
    pub cache: Option<&'a Cache>,
}

/// The third-party content a flavor's own loader consumes: mods for a
/// modloader, plugins for a server platform, nothing for vanilla. Datapacks
/// (and, client-side, resourcepacks and shaders) belong to the game rather than
/// the flavor, so they are not named here — the content flows add them.
pub type Loads = Option<ContentKind>;

#[async_trait]
pub trait ServerProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn loads(&self) -> Loads;
    async fn versions(&self) -> Result<Vec<GameVersion>>;
    async fn resolve(&self, request: &ResolveRequest) -> Result<ServerProfile>;
    /// The loader builds available for a game version, newest first. A flavor
    /// with no loader concept (vanilla) reports none — the default.
    async fn loader_versions(&self, _game: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// See [`InstanceProvider::install`].
    async fn install(&self, _request: &InstallRequest<'_>, _on: OnProgress<'_>) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
pub trait InstanceProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn loads(&self) -> Loads;
    async fn versions(&self) -> Result<Vec<GameVersion>>;
    async fn resolve(&self, request: &ResolveRequest) -> Result<InstanceProfile>;
    /// The loader builds available for a game version, newest first. A flavor
    /// with no loader concept (vanilla) reports none — the default.
    async fn loader_versions(&self, _game: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// Build whatever the profile cannot name — the patched jar a NeoForge
    /// client runs, which exists nowhere to download. Idempotent: it runs on
    /// every launch and must cost nothing once done.
    async fn install(&self, _request: &InstallRequest<'_>, _on: OnProgress<'_>) -> Result<()> {
        Ok(())
    }
}
