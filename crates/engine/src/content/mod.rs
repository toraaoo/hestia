//! Third-party content provider aggregate: the source registry (Modrinth and
//! CurseForge behind one trait) and the search/project/versions/modpack entry
//! points over it. Stateless — every result is fetched upstream — so it needs no
//! data directory, exactly like the `minecraft` aggregate. The only state is the
//! per-source configuration `configure()` hands down, since CurseForge serves
//! nothing without an API key.

pub(crate) mod curseforge;
pub(crate) mod exclude;
pub(crate) mod inspect;
pub(crate) mod install;
pub(crate) mod modpack;
mod modrinth;
pub(crate) mod pack;
pub(crate) mod profiles;
pub(crate) mod provider;

use std::path::Path;

use anyhow::{Context, Result};
use proto::content::{
    ContentInspectResult, ContentKind, ContentProject, ContentSource, ContentVersion,
    ResolvedModpack, ResolvedUrl, SearchQuery, SearchResult, VersionQuery,
};

use crate::config::ContentSettings;
use provider::ContentProvider;
pub(crate) use provider::UrlRef;

/// Bring an entry's content documents forward. Reading one is what migrates it,
/// so this is a read of each — called after an import lands a tree written by
/// another build, rather than waiting for the first thing that happens to ask.
pub(crate) fn migrate(entry_dir: &Path) {
    install::load(entry_dir);
    profiles::migrate(entry_dir);
    modpack::load(entry_dir);
}

pub struct Content {
    providers: Vec<Box<dyn ContentProvider>>,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            providers: vec![
                Box::new(modrinth::Modrinth),
                Box::new(curseforge::CurseForge::default()),
            ],
        }
    }
}

impl Content {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build over a given registry rather than the shipped one. The seam a test
    /// crosses to install content without reaching a platform.
    pub fn with_providers(providers: Vec<Box<dyn ContentProvider>>) -> Self {
        Content { providers }
    }

    /// Hand each source the settings it needs. Called at startup and after
    /// every `config set`, so a key takes effect on the running daemon.
    pub fn configure(&self, settings: &ContentSettings) {
        for provider in &self.providers {
            provider.configure(settings);
        }
    }

    /// Classify a local file for import (the daemon is the only side that can
    /// read a client-picked path).
    pub fn inspect(&self, path: &str) -> ContentInspectResult {
        let p = Path::new(path);
        let filename = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !p.is_file() {
            return ContentInspectResult {
                valid: false,
                kind: None,
                filename,
                reason: format!("no file at {}", p.display()),
            };
        }
        match inspect::classify(p) {
            Ok(inspect::Detected::Kind(kind)) => ContentInspectResult {
                valid: true,
                kind: Some(kind),
                filename,
                reason: String::new(),
            },
            Ok(inspect::Detected::Unknown) => ContentInspectResult {
                valid: true,
                kind: None,
                filename,
                reason: String::new(),
            },
            Err(e) => ContentInspectResult {
                valid: false,
                kind: None,
                filename,
                reason: format!("{e:#}"),
            },
        }
    }

    /// The sources that can actually serve a request — a platform whose API key
    /// is unset is registered but not offered.
    pub fn sources(&self) -> Vec<ContentSource> {
        self.providers
            .iter()
            .filter(|p| p.available())
            .map(|p| ContentSource {
                id: p.id().to_string(),
                name: p.name().to_string(),
                kinds: p.kinds(),
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

    pub async fn project(
        &self,
        source: &str,
        project: &str,
        kind: Option<ContentKind>,
    ) -> Result<ContentProject> {
        let provider = self.provider(source)?;
        tracing::info!(
            source = provider.id(),
            project,
            ?kind,
            "content project lookup"
        );
        provider.project(project, kind).await
    }

    /// Resolve a source page URL to the project it names (and the version it
    /// pins, for a version page).
    pub async fn resolve_url(&self, url: &str) -> Result<ResolvedUrl> {
        let (source, reference) = self.parse_url(url).ok_or_else(|| {
            anyhow::Error::from(proto::error::ErrorInfo::UnsupportedContentUrl {
                url: url.to_string(),
            })
        })?;
        let project = self
            .project(&source, &reference.project, reference.kind)
            .await?;
        Ok(ResolvedUrl {
            project,
            version_id: reference.version.unwrap_or_default(),
        })
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

    /// The pack version's whole archive alongside its manifest — what installing
    /// needs, since a pack's `overrides/` exist only inside the zip.
    pub async fn fetch_modpack(
        &self,
        source: &str,
        version_id: &str,
    ) -> Result<(ResolvedModpack, Vec<u8>)> {
        let provider = self.provider(source)?;
        tracing::info!(source = provider.id(), version_id, "modpack fetch");
        let (resolved, bytes) = provider.fetch_modpack(version_id).await?;
        tracing::info!(
            source = %resolved.source,
            version_id = %resolved.version_id,
            files = resolved.files.len(),
            bytes = bytes.len(),
            game_version = %resolved.game_version,
            loader = ?resolved.loader,
            "modpack fetched"
        );
        Ok((resolved, bytes))
    }

    /// Several projects at once — one request where the provider has a bulk
    /// endpoint, which is what keeps a pack's hundred-odd lookups affordable.
    pub async fn projects(&self, source: &str, ids: &[String]) -> Result<Vec<ContentProject>> {
        self.provider(source)?.projects(ids).await
    }

    /// Several versions by id — the bulk twin of [`Content::projects`].
    pub async fn versions_by_id(
        &self,
        source: &str,
        ids: &[String],
    ) -> Result<Vec<ContentVersion>> {
        self.provider(source)?.versions_by_id(ids).await
    }

    /// What a platform's own download URL says about the file behind it, so a
    /// pack index's bare URLs become tracked pool items.
    pub(crate) fn parse_file_url(&self, source: &str, url: &str) -> Option<provider::FileRef> {
        self.provider(source).ok()?.parse_file_url(url)
    }

    /// Recognise a project/version page URL on any registered platform's site,
    /// returning the owning source id and the reference it names.
    pub(crate) fn parse_url(&self, url: &str) -> Option<(String, UrlRef)> {
        self.providers
            .iter()
            .find_map(|p| p.parse_url(url).map(|r| (p.id().to_string(), r)))
    }

    /// The provider for `id`; an empty id selects the default — the first
    /// source that can serve. A source that is registered but unconfigured is
    /// refused by name rather than reported unknown, since the difference is
    /// what the user has to act on.
    fn provider(&self, id: &str) -> Result<&dyn ContentProvider> {
        if id.is_empty() {
            return self
                .providers
                .iter()
                .map(AsRef::as_ref)
                .find(|p| p.available())
                .context("no content source is configured");
        }
        let provider = self
            .providers
            .iter()
            .map(AsRef::as_ref)
            .find(|p| p.id() == id)
            .with_context(|| format!("unknown content source: {id}"))?;
        if !provider.available() {
            return Err(proto::error::ErrorInfo::ContentSourceUnavailable {
                source: provider.id().to_string(),
            }
            .into());
        }
        Ok(provider)
    }
}
