//! The content-provider seam. A source platform implements the trait — listing,
//! searching, and resolving projects and their versions — and the `Content`
//! aggregate holds a boxed registry of each. Adding a platform is a new impl plus
//! one line in `Content::new`.

use anyhow::Result;
use async_trait::async_trait;
use proto::content::{ContentProject, ContentVersion, ResolvedModpack, SearchQuery, VersionQuery};

#[async_trait]
pub trait ContentProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn search(&self, query: &SearchQuery) -> Result<proto::content::SearchResult>;
    async fn project(&self, project: &str) -> Result<ContentProject>;
    async fn versions(&self, query: &VersionQuery) -> Result<Vec<ContentVersion>>;
    async fn resolve_modpack(&self, version_id: &str) -> Result<ResolvedModpack>;
}
