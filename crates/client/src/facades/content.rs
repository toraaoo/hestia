use std::time::Duration;

use ipc::errors::IpcError;
use proto::content::{
    ContentProject, ContentProjectGet, ContentSearch, ContentSource, ContentSources,
    ContentVersion, ContentVersions, ModpackParams, ModpackResolve, ProjectParams, ResolvedModpack,
    SearchQuery, SearchResult, VersionQuery,
};

use crate::session::Session;

pub struct Content<'a> {
    pub(crate) session: &'a Session,
}

impl Content<'_> {
    /// The available content sources (modrinth, …).
    pub async fn sources(&self) -> Result<Vec<ContentSource>, IpcError> {
        Ok(self
            .session
            .call::<ContentSources>(&proto::Empty {})
            .await?
            .sources)
    }

    /// A paginated search over a source (empty `query.source` picks the default).
    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResult, IpcError> {
        self.session.call::<ContentSearch>(query).await
    }

    pub async fn project(&self, source: &str, project: &str) -> Result<ContentProject, IpcError> {
        let params = ProjectParams {
            source: source.to_string(),
            project: project.to_string(),
        };
        self.session.call::<ContentProjectGet>(&params).await
    }

    pub async fn versions(&self, query: &VersionQuery) -> Result<Vec<ContentVersion>, IpcError> {
        Ok(self.session.call::<ContentVersions>(query).await?.versions)
    }

    /// Resolve a modpack version into its file manifest. Downloads the `.mrpack`
    /// index on the daemon, so it carries a longer timeout.
    pub async fn resolve_modpack(
        &self,
        source: &str,
        version_id: &str,
    ) -> Result<ResolvedModpack, IpcError> {
        let params = ModpackParams {
            source: source.to_string(),
            version_id: version_id.to_string(),
        };
        self.session
            .call_with_timeout::<ModpackResolve>(&params, Duration::from_secs(120))
            .await
    }
}
