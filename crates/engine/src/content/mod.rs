//! Third-party content provider aggregate: the source registry (Modrinth today,
//! CurseForge behind the same trait later) and the search/project/versions/
//! modpack entry points over it. Stateless — every result is fetched upstream —
//! so it needs no data directory, exactly like the `minecraft` aggregate.

pub(crate) mod install;
mod modrinth;
mod provider;

use anyhow::{Context, Result};
use proto::content::{
    ContentProject, ContentSource, ContentVersion, ResolvedModpack, SearchQuery, SearchResult,
    VersionQuery,
};

use provider::ContentProvider;
pub(crate) use provider::UrlRef;

pub struct Content {
    providers: Vec<Box<dyn ContentProvider>>,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            providers: vec![Box::new(modrinth::Modrinth)],
        }
    }
}

impl Content {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sources(&self) -> Vec<ContentSource> {
        self.providers
            .iter()
            .map(|p| ContentSource {
                id: p.id().to_string(),
                name: p.name().to_string(),
            })
            .collect()
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let provider = self.provider(&query.source)?;
        tracing::info!(
            source = provider.id(),
            kind = ?query.kind,
            query = %query.query,
            offset = query.offset,
            limit = query.limit,
            "content search"
        );
        provider.search(query).await
    }

    pub async fn project(&self, source: &str, project: &str) -> Result<ContentProject> {
        let provider = self.provider(source)?;
        tracing::info!(source = provider.id(), project, "content project lookup");
        provider.project(project).await
    }

    pub async fn versions(&self, query: &VersionQuery) -> Result<Vec<ContentVersion>> {
        let provider = self.provider(&query.source)?;
        tracing::info!(
            source = provider.id(),
            project = %query.project,
            loader = ?query.loader,
            game_version = ?query.game_version,
            "content versions lookup"
        );
        provider.versions(query).await
    }

    pub async fn resolve_modpack(&self, source: &str, version_id: &str) -> Result<ResolvedModpack> {
        let provider = self.provider(source)?;
        tracing::info!(source = provider.id(), version_id, "modpack resolve");
        let resolved = provider.resolve_modpack(version_id).await?;
        tracing::info!(
            source = %resolved.source,
            version_id = %resolved.version_id,
            files = resolved.files.len(),
            game_version = %resolved.game_version,
            loader = ?resolved.loader,
            "modpack resolved"
        );
        Ok(resolved)
    }

    /// Recognise a project/version page URL on any registered platform's site,
    /// returning the owning source id and the reference it names.
    pub(crate) fn parse_url(&self, url: &str) -> Option<(String, UrlRef)> {
        self.providers
            .iter()
            .find_map(|p| p.parse_url(url).map(|r| (p.id().to_string(), r)))
    }

    /// The provider for `id`; an empty id selects the default (first) source.
    fn provider(&self, id: &str) -> Result<&dyn ContentProvider> {
        if id.is_empty() {
            return self
                .providers
                .first()
                .map(AsRef::as_ref)
                .context("no content providers are registered");
        }
        self.providers
            .iter()
            .map(AsRef::as_ref)
            .find(|p| p.id() == id)
            .with_context(|| format!("unknown content source: {id}"))
    }
}
