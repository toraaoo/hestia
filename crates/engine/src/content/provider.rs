//! The content-provider seam. A source platform implements the trait — listing,
//! searching, and resolving projects and their versions — and the `Content`
//! aggregate holds a boxed registry of each. Adding a platform is a new impl plus
//! one line in `Content::new`.

use anyhow::Result;
use async_trait::async_trait;
use proto::content::{ContentProject, ContentVersion, ResolvedModpack, SearchQuery, VersionQuery};

/// A project reference recognised in a platform's own site URL, optionally
/// pinned to one version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlRef {
    pub project: String,
    pub version: Option<String>,
}

#[async_trait]
pub trait ContentProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// Recognise a project/version page URL on this platform's site.
    fn parse_url(&self, url: &str) -> Option<UrlRef>;
    async fn search(&self, query: &SearchQuery) -> Result<proto::content::SearchResult>;
    async fn project(&self, project: &str) -> Result<ContentProject>;
    async fn versions(&self, query: &VersionQuery) -> Result<Vec<ContentVersion>>;
    async fn resolve_modpack(&self, version_id: &str) -> Result<ResolvedModpack>;
}
