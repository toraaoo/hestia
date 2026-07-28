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
/// the flavor, so they are not named here — [`accepted_kinds`] adds them.
pub type Loads = Option<ContentKind>;

/// Which half of the game an entry is, for the rules that differ between them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Server,
    Client,
}

/// Everything an entry of this flavor and side accepts, composed from two
/// independent facts: whatever the flavor's own loader takes, plus what the
/// side reads for itself — a client its resourcepacks and shaders, either side
/// the datapacks that are world data rather than loader content.
///
/// It lives beside `Loads` because it is the same vocabulary: adding a flavor
/// is one impl plus one registry line, with no table to update here or in any
/// front-end.
pub fn accepted_kinds(side: Side, loads: Loads) -> Vec<ContentKind> {
    let mut kinds: Vec<ContentKind> = loads.into_iter().collect();
    if side == Side::Client {
        kinds.push(ContentKind::ResourcePack);
        kinds.push(ContentKind::Shader);
    }
    kinds.push(ContentKind::DataPack);
    kinds
}

#[async_trait]
pub trait ServerProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// One line describing the distribution — what it is, and anything about
    /// running it a user should know before choosing it. Rendered by every
    /// front-end, so a new flavor needs no front-end change to explain itself.
    fn summary(&self) -> &'static str;
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
    /// See [`ServerProvider::summary`].
    fn summary(&self) -> &'static str;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flavor_contributes_its_loader_kind_and_the_side_the_rest() {
        assert_eq!(
            accepted_kinds(Side::Server, Some(ContentKind::Plugin)),
            vec![ContentKind::Plugin, ContentKind::DataPack],
            "a paper or spigot server takes plugins, never mods"
        );
        assert_eq!(
            accepted_kinds(Side::Server, Some(ContentKind::Mod)),
            vec![ContentKind::Mod, ContentKind::DataPack]
        );
        assert_eq!(
            accepted_kinds(Side::Server, None),
            vec![ContentKind::DataPack],
            "vanilla loads nothing of its own, but a world still takes datapacks"
        );
        assert_eq!(
            accepted_kinds(Side::Client, None),
            vec![
                ContentKind::ResourcePack,
                ContentKind::Shader,
                ContentKind::DataPack
            ],
            "a client reads packs whatever its flavor loads"
        );
    }
}
