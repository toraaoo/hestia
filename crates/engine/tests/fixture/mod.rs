//! Stand-in flavors and content sources. The provider traits are the engine's
//! real seams; these are the second adapter behind them, so a flow can be
//! driven without reaching Mojang, Modrinth or a JDK mirror.

use anyhow::{bail, Result};
use async_trait::async_trait;
use engine::{Content, ContentProvider, InstanceProvider, Minecraft, ServerProvider, UrlRef};
use proto::content::{
    ContentKind, ContentProject, ContentVersion, ResolvedModpack, SearchQuery, SearchResult,
    VersionQuery,
};
use proto::minecraft::{Artifact, GameVersion, InstanceProfile, ServerProfile};

/// A flavor that resolves to a profile naming `artifact`, or fails when it is
/// `None` — the two halves of what a provider can do to a create.
pub struct Flavor {
    pub id: &'static str,
    pub versions: Vec<&'static str>,
    pub artifact: Option<Artifact>,
}

impl Flavor {
    pub fn resolving(id: &'static str) -> Flavor {
        Flavor {
            id,
            versions: vec!["1.21", "1.20.1"],
            artifact: Some(Artifact {
                url: "http://127.0.0.1:1/server.jar".into(),
                filename: "server.jar".into(),
                size: 0,
                checksum: None,
            }),
        }
    }

    pub fn failing(id: &'static str) -> Flavor {
        Flavor {
            id,
            versions: vec!["1.21"],
            artifact: None,
        }
    }

    fn catalogue(&self) -> Vec<GameVersion> {
        self.versions
            .iter()
            .map(|v| GameVersion {
                id: (*v).to_string(),
                ..GameVersion::default()
            })
            .collect()
    }

    fn primary(&self) -> Result<Artifact> {
        match &self.artifact {
            Some(artifact) => Ok(artifact.clone()),
            None => bail!("this flavor cannot resolve"),
        }
    }
}

#[async_trait]
impl ServerProvider for Flavor {
    fn id(&self) -> &'static str {
        self.id
    }
    fn name(&self) -> &'static str {
        "Fixture"
    }
    fn summary(&self) -> &'static str {
        "a flavor that exists only in tests"
    }
    fn loads(&self) -> engine::Loads {
        Some(ContentKind::Mod)
    }
    async fn versions(&self) -> Result<Vec<GameVersion>> {
        Ok(self.catalogue())
    }
    async fn resolve(&self, request: &engine::ResolveRequest) -> Result<ServerProfile> {
        Ok(ServerProfile {
            flavor: self.id.to_string(),
            game_version: request.version.clone(),
            primary: self.primary()?,
            java_major: 21,
            main_class: "net.minecraft.server.Main".into(),
            ..ServerProfile::default()
        })
    }
}

#[async_trait]
impl InstanceProvider for Flavor {
    fn id(&self) -> &'static str {
        self.id
    }
    fn name(&self) -> &'static str {
        "Fixture"
    }
    fn summary(&self) -> &'static str {
        "a flavor that exists only in tests"
    }
    fn loads(&self) -> engine::Loads {
        Some(ContentKind::Mod)
    }
    async fn versions(&self) -> Result<Vec<GameVersion>> {
        Ok(self.catalogue())
    }
    async fn resolve(&self, request: &engine::ResolveRequest) -> Result<InstanceProfile> {
        Ok(InstanceProfile {
            flavor: self.id.to_string(),
            game_version: request.version.clone(),
            client: self.primary()?,
            java_major: 21,
            main_class: "net.minecraft.client.main.Main".into(),
            ..InstanceProfile::default()
        })
    }
}

/// A content source that serves whatever it was built with.
#[derive(Default)]
pub struct Source {
    pub projects: Vec<ContentProject>,
    pub versions: Vec<ContentVersion>,
}

#[async_trait]
impl ContentProvider for Source {
    fn id(&self) -> &'static str {
        "fixture"
    }
    fn name(&self) -> &'static str {
        "Fixture"
    }
    fn kinds(&self) -> Vec<ContentKind> {
        vec![ContentKind::Mod, ContentKind::DataPack]
    }
    async fn search(&self, _query: &SearchQuery) -> Result<SearchResult> {
        Ok(SearchResult {
            hits: self.projects.clone(),
            total: self.projects.len() as u32,
            ..SearchResult::default()
        })
    }
    async fn project(&self, reference: &str, _kind: Option<ContentKind>) -> Result<ContentProject> {
        self.projects
            .iter()
            .find(|p| p.id == reference || p.slug == reference)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no such project"))
    }
    async fn versions(&self, _query: &VersionQuery) -> Result<Vec<ContentVersion>> {
        Ok(self.versions.clone())
    }
    fn parse_url(&self, _url: &str) -> Option<UrlRef> {
        None
    }
    async fn resolve_modpack(&self, _version: &str) -> Result<ResolvedModpack> {
        bail!("the fixture source serves no packs")
    }
    async fn fetch_modpack(&self, _version_id: &str) -> Result<(ResolvedModpack, Vec<u8>)> {
        bail!("the fixture source serves no packs")
    }
}

/// An engine over `flavors` for both sides and `sources` for content.
pub fn engine(
    home: &std::path::Path,
    flavors: Vec<Flavor>,
    sources: Vec<Source>,
) -> engine::Engine {
    let servers: Vec<Box<dyn ServerProvider>> = flavors
        .iter()
        .map(|f| Box::new(Flavor { ..clone_of(f) }) as Box<dyn ServerProvider>)
        .collect();
    let instances: Vec<Box<dyn InstanceProvider>> = flavors
        .iter()
        .map(|f| Box::new(clone_of(f)) as Box<dyn InstanceProvider>)
        .collect();
    let providers: Vec<Box<dyn ContentProvider>> = sources
        .into_iter()
        .map(|s| Box::new(s) as Box<dyn ContentProvider>)
        .collect();
    engine::Engine::over(
        Some(home),
        Minecraft::with_providers(servers, instances),
        Content::with_providers(providers),
    )
}

fn clone_of(flavor: &Flavor) -> Flavor {
    Flavor {
        id: flavor.id,
        versions: flavor.versions.clone(),
        artifact: flavor.artifact.clone(),
    }
}
