//! The Modrinth (`api.modrinth.com/v2`) content provider: search with facets,
//! project detail, project versions, and `.mrpack` modpack resolution. Modrinth's
//! raw JSON is mapped into the normalized `proto::content` types here; the rest of
//! the engine never sees a Modrinth-specific shape. No API key is required.

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use proto::content::{
    ContentDependency, ContentFile, ContentKind, ContentProject, ContentVersion, DependencyKind,
    GalleryImage, ModpackFile, ReleaseChannel, ResolvedModpack, SearchQuery, SearchResult,
    SearchSort, SideSupport, VersionQuery,
};
use proto::download::{Checksum, HashAlgorithm};
use proto::minecraft::Artifact;
use serde_json::{Map, Value};
use std::io::Cursor;
use std::path::{Component, Path};

use super::provider::{ContentProvider, UrlRef};

const API: &str = "https://api.modrinth.com/v2";
const SITE: &str = "modrinth.com";

/// Modrinth dependency keys that name a modloader, newest-preferred order. The
/// loader name is the key with any `-loader` suffix stripped.
const LOADER_KEYS: [&str; 4] = ["fabric-loader", "quilt-loader", "neoforge", "forge"];

/// The site's project-type path segments (`modrinth.com/<type>/<slug>`).
const SITE_TYPES: [&str; 6] = [
    "mod",
    "modpack",
    "resourcepack",
    "shader",
    "datapack",
    "plugin",
];

pub struct Modrinth;

#[async_trait]
impl ContentProvider for Modrinth {
    fn id(&self) -> &'static str {
        "modrinth"
    }

    fn name(&self) -> &'static str {
        "Modrinth"
    }

    /// `modrinth.com/<type>/<slug>` names a project;
    /// `…/<slug>/version/<number-or-id>` pins one of its versions.
    fn parse_url(&self, url: &str) -> Option<UrlRef> {
        let rest = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))?;
        let rest = rest.strip_prefix("www.").unwrap_or(rest);
        let path = rest.strip_prefix(SITE)?.strip_prefix('/')?;
        let mut segments = path
            .split(['?', '#'])
            .next()?
            .split('/')
            .filter(|s| !s.is_empty());
        let kind = segments.next()?;
        if !SITE_TYPES.contains(&kind) {
            return None;
        }
        let project = segments.next()?.to_string();
        let version = match (segments.next(), segments.next()) {
            (Some("version"), Some(v)) => Some(v.to_string()),
            _ => None,
        };
        Some(UrlRef { project, version })
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let limit = if query.limit == 0 {
            10
        } else {
            query.limit.clamp(1, 100)
        };
        let mut params: Vec<(&str, String)> = vec![
            ("facets", build_facets(query)),
            ("index", sort_index(query.sort).to_string()),
            ("offset", query.offset.to_string()),
            ("limit", limit.to_string()),
        ];
        if !query.query.is_empty() {
            params.push(("query", query.query.clone()));
        }
        let root = get_json(&format!("{API}/search"), &params).await?;
        let hits = root
            .get("hits")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|h| parse_hit(self.id(), h)).collect())
            .unwrap_or_default();
        Ok(SearchResult {
            hits,
            offset: root
                .get("offset")
                .and_then(Value::as_u64)
                .unwrap_or(query.offset as u64) as u32,
            limit,
            total: root.get("total_hits").and_then(Value::as_u64).unwrap_or(0) as u32,
        })
    }

    async fn project(&self, project: &str) -> Result<ContentProject> {
        let body = get_json(&format!("{API}/project/{project}"), &[]).await?;
        Ok(parse_project(self.id(), &body))
    }

    async fn versions(&self, query: &VersionQuery) -> Result<Vec<ContentVersion>> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(loader) = non_empty(&query.loader) {
            params.push(("loaders", json_array(loader)));
        }
        if let Some(game) = non_empty(&query.game_version) {
            params.push(("game_versions", json_array(game)));
        }
        let arr = get_json(&format!("{API}/project/{}/version", query.project), &params).await?;
        Ok(arr
            .as_array()
            .map(|a| a.iter().map(|v| parse_version(self.id(), v)).collect())
            .unwrap_or_default())
    }

    async fn resolve_modpack(&self, version_id: &str) -> Result<ResolvedModpack> {
        let version = get_json(&format!("{API}/version/{version_id}"), &[]).await?;
        let files = version
            .get("files")
            .and_then(Value::as_array)
            .filter(|f| !f.is_empty())
            .context("modpack version has no files")?;
        let file = files
            .iter()
            .find(|f| f.get("primary").and_then(Value::as_bool).unwrap_or(false))
            .unwrap_or(&files[0]);
        let filename = file
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !filename.ends_with(".mrpack") {
            bail!("version {version_id} is not a Modrinth modpack (expected a .mrpack file)");
        }
        let url = file
            .get("url")
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
            .context("modpack file has no download url")?;

        let bytes = download_bytes(url).await?;
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .context("the modpack .mrpack is not a valid archive")?;
        let index: Value = {
            let entry = archive
                .by_name("modrinth.index.json")
                .context("modrinth.index.json is missing from the .mrpack")?;
            serde_json::from_reader(entry).context("modrinth.index.json is malformed")?
        };

        let mut resolved = parse_index(&index)?;
        resolved.source = self.id().to_string();
        resolved.version_id = version
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or(version_id)
            .to_string();
        resolved.project_id = version
            .get("project_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(resolved)
    }
}

/// Parse a `modrinth.index.json` (the `.mrpack` manifest) into a resolved
/// modpack. Pure — the source/project/version ids come from the API version
/// response, not the index. Rejects an unsupported format version, a file with
/// an unsafe (absolute or parent-escaping) path, or a missing Minecraft version.
fn parse_index(index: &Value) -> Result<ResolvedModpack> {
    let format = index
        .get("formatVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if format != 1 {
        bail!("unsupported modpack format version: {format} (expected 1)");
    }

    let mut files = Vec::new();
    if let Some(arr) = index.get("files").and_then(Value::as_array) {
        for f in arr {
            let path = f.get("path").and_then(Value::as_str).unwrap_or_default();
            if !is_safe_path(path) {
                bail!("modpack file has an unsafe path: {path}");
            }
            let url = f
                .get("downloads")
                .and_then(Value::as_array)
                .and_then(|d| d.first())
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let size = f.get("fileSize").and_then(Value::as_u64).unwrap_or(0);
            let sha1 = f
                .get("hashes")
                .and_then(|h| h.get("sha1"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let env = f.get("env");
            // A missing `env` means the file applies to both sides (Modrinth spec).
            let client = env
                .and_then(|e| e.get("client"))
                .and_then(Value::as_str)
                .map(parse_side)
                .unwrap_or(SideSupport::Required);
            let server = env
                .and_then(|e| e.get("server"))
                .and_then(Value::as_str)
                .map(parse_side)
                .unwrap_or(SideSupport::Required);
            files.push(ModpackFile {
                path: path.to_string(),
                artifact: Artifact {
                    url,
                    filename: filename_of(path),
                    size,
                    checksum: (!sha1.is_empty()).then_some(Checksum {
                        algorithm: HashAlgorithm::Sha1,
                        hex: sha1,
                    }),
                },
                client,
                server,
            });
        }
    }

    let deps = index.get("dependencies").and_then(Value::as_object);
    let game_version = deps
        .and_then(|d| d.get("minecraft"))
        .and_then(Value::as_str)
        .context("modpack does not pin a Minecraft version")?
        .to_string();
    let (loader, loader_version) = deps
        .and_then(find_loader)
        .map(|(l, v)| (Some(l), Some(v)))
        .unwrap_or((None, None));

    Ok(ResolvedModpack {
        source: String::new(),
        project_id: String::new(),
        version_id: String::new(),
        name: index
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        game_version,
        loader,
        loader_version,
        files,
    })
}

fn parse_hit(source: &str, hit: &Value) -> ContentProject {
    ContentProject {
        source: source.to_string(),
        id: str_field(hit, "project_id"),
        slug: str_field(hit, "slug"),
        kind: parse_kind(&str_field(hit, "project_type")),
        title: str_field(hit, "title"),
        description: str_field(hit, "description"),
        body: String::new(),
        author: str_field(hit, "author"),
        categories: categories(hit),
        downloads: u64_field(hit, "downloads"),
        follows: u64_field(hit, "follows"),
        icon_url: str_field(hit, "icon_url"),
        gallery: hit
            .get("gallery")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(|url| GalleryImage {
                        url: url.to_string(),
                        ..GalleryImage::default()
                    })
                    .collect()
            })
            .unwrap_or_default(),
        client_side: parse_side(&str_field(hit, "client_side")),
        server_side: parse_side(&str_field(hit, "server_side")),
    }
}

fn parse_project(source: &str, body: &Value) -> ContentProject {
    ContentProject {
        source: source.to_string(),
        id: str_field(body, "id"),
        slug: str_field(body, "slug"),
        kind: parse_kind(&str_field(body, "project_type")),
        title: str_field(body, "title"),
        description: str_field(body, "description"),
        body: str_field(body, "body"),
        author: String::new(),
        categories: categories(body),
        downloads: u64_field(body, "downloads"),
        follows: u64_field(body, "follows"),
        icon_url: str_field(body, "icon_url"),
        gallery: body
            .get("gallery")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|g| GalleryImage {
                        url: str_field(g, "url"),
                        featured: g.get("featured").and_then(Value::as_bool).unwrap_or(false),
                        title: str_field(g, "title"),
                        description: str_field(g, "description"),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        client_side: parse_side(&str_field(body, "client_side")),
        server_side: parse_side(&str_field(body, "server_side")),
    }
}

fn parse_version(source: &str, v: &Value) -> ContentVersion {
    let files = v
        .get("files")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|f| ContentFile {
                    artifact: Artifact {
                        url: str_field(f, "url"),
                        filename: str_field(f, "filename"),
                        size: u64_field(f, "size"),
                        checksum: f
                            .get("hashes")
                            .and_then(|h| h.get("sha1"))
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .map(|hex| Checksum {
                                algorithm: HashAlgorithm::Sha1,
                                hex: hex.to_string(),
                            }),
                    },
                    primary: f.get("primary").and_then(Value::as_bool).unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default();
    let dependencies = v
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|d| ContentDependency {
                    project_id: str_field(d, "project_id"),
                    version_id: str_field(d, "version_id"),
                    kind: parse_dependency_kind(&str_field(d, "dependency_type")),
                })
                .collect()
        })
        .unwrap_or_default();
    ContentVersion {
        source: source.to_string(),
        id: str_field(v, "id"),
        project_id: str_field(v, "project_id"),
        name: str_field(v, "name"),
        version_number: str_field(v, "version_number"),
        channel: parse_channel(&str_field(v, "version_type")),
        game_versions: str_array(v, "game_versions"),
        loaders: str_array(v, "loaders"),
        featured: v.get("featured").and_then(Value::as_bool).unwrap_or(false),
        date_published: str_field(v, "date_published"),
        downloads: u64_field(v, "downloads"),
        files,
        dependencies,
    }
}

/// Modrinth facets are a JSON array of single-element arrays (each AND'd). The
/// loader is expressed as a `categories:` facet, as Modrinth does. Only set
/// filters are included.
fn build_facets(query: &SearchQuery) -> String {
    let mut facets: Vec<Vec<String>> =
        vec![vec![format!("project_type:{}", project_type(query.kind))]];
    if let Some(loader) = non_empty(&query.loader) {
        facets.push(vec![format!("categories:{loader}")]);
    }
    if let Some(game) = non_empty(&query.game_version) {
        facets.push(vec![format!("versions:{game}")]);
    }
    for category in &query.categories {
        if !category.is_empty() {
            facets.push(vec![format!("categories:{category}")]);
        }
    }
    serde_json::to_string(&facets).unwrap_or_default()
}

fn project_type(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mod",
        ContentKind::Modpack => "modpack",
        ContentKind::ResourcePack => "resourcepack",
        ContentKind::Shader => "shader",
        ContentKind::DataPack => "datapack",
    }
}

fn parse_kind(s: &str) -> ContentKind {
    match s {
        "modpack" => ContentKind::Modpack,
        "resourcepack" => ContentKind::ResourcePack,
        "shader" => ContentKind::Shader,
        "datapack" => ContentKind::DataPack,
        _ => ContentKind::Mod,
    }
}

fn sort_index(sort: SearchSort) -> &'static str {
    match sort {
        SearchSort::Relevance => "relevance",
        SearchSort::Downloads => "downloads",
        SearchSort::Follows => "follows",
        SearchSort::Newest => "newest",
        SearchSort::Updated => "updated",
    }
}

fn parse_side(s: &str) -> SideSupport {
    match s {
        "required" => SideSupport::Required,
        "optional" => SideSupport::Optional,
        "unsupported" => SideSupport::Unsupported,
        _ => SideSupport::Unknown,
    }
}

fn parse_channel(s: &str) -> ReleaseChannel {
    match s {
        "beta" => ReleaseChannel::Beta,
        "alpha" => ReleaseChannel::Alpha,
        _ => ReleaseChannel::Release,
    }
}

fn parse_dependency_kind(s: &str) -> DependencyKind {
    match s {
        "optional" => DependencyKind::Optional,
        "incompatible" => DependencyKind::Incompatible,
        "embedded" => DependencyKind::Embedded,
        _ => DependencyKind::Required,
    }
}

fn find_loader(deps: &Map<String, Value>) -> Option<(String, String)> {
    for key in LOADER_KEYS {
        if let Some(version) = deps.get(key).and_then(Value::as_str) {
            let name = key.strip_suffix("-loader").unwrap_or(key).to_string();
            return Some((name, version.to_string()));
        }
    }
    None
}

/// A relative path that stays inside the game directory: not empty, not
/// absolute, and with no parent (`..`), root, or drive-prefix components.
fn is_safe_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = Path::new(path);
    if p.is_absolute() {
        return false;
    }
    p.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn categories(v: &Value) -> Vec<String> {
    let display = str_array(v, "display_categories");
    if display.is_empty() {
        str_array(v, "categories")
    } else {
        display
    }
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn u64_field(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn str_array(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn non_empty(opt: &Option<String>) -> Option<&str> {
    opt.as_deref().filter(|s| !s.is_empty())
}

fn json_array(value: &str) -> String {
    serde_json::to_string(&[value]).unwrap_or_default()
}

fn filename_of(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

async fn get_json(url: &str, query: &[(&str, String)]) -> Result<Value> {
    tracing::debug!(url, "modrinth GET");
    let response = crate::download::http_client()
        .get(url)
        .query(query)
        .send()
        .await
        .with_context(|| format!("request to {url} failed"))?;
    if !response.status().is_success() {
        bail!(
            "request to {url} failed: HTTP {}",
            response.status().as_u16()
        );
    }
    response
        .json()
        .await
        .with_context(|| format!("{url} returned malformed JSON"))
}

async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    tracing::debug!(url, "modrinth modpack GET");
    let response = crate::download::http_client()
        .get(url)
        .send()
        .await
        .with_context(|| format!("download of {url} failed"))?;
    if !response.status().is_success() {
        bail!(
            "download of {url} failed: HTTP {}",
            response.status().as_u16()
        );
    }
    Ok(response
        .bytes()
        .await
        .with_context(|| format!("reading {url} failed"))?
        .to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SHA1: &str = "da39a3ee5e6b4b0d3255bfef95601890afd80709";

    #[test]
    fn parse_index_maps_files_and_loader() {
        let index = json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.2.3",
            "name": "Test Pack",
            "files": [{
                "path": "mods/sodium.jar",
                "hashes": { "sha1": SHA1, "sha512": "ignored" },
                "env": { "client": "required", "server": "unsupported" },
                "downloads": ["https://cdn.modrinth.com/sodium.jar"],
                "fileSize": 1234
            }],
            "dependencies": { "minecraft": "1.21.1", "fabric-loader": "0.16.0" }
        });
        let resolved = parse_index(&index).unwrap();
        assert_eq!(resolved.name, "Test Pack");
        assert_eq!(resolved.game_version, "1.21.1");
        assert_eq!(resolved.loader.as_deref(), Some("fabric"));
        assert_eq!(resolved.loader_version.as_deref(), Some("0.16.0"));
        assert_eq!(resolved.files.len(), 1);
        let file = &resolved.files[0];
        assert_eq!(file.path, "mods/sodium.jar");
        assert_eq!(file.artifact.filename, "sodium.jar");
        assert_eq!(file.artifact.url, "https://cdn.modrinth.com/sodium.jar");
        assert_eq!(file.artifact.size, 1234);
        assert_eq!(file.artifact.checksum.as_ref().unwrap().hex, SHA1);
        assert_eq!(file.client, SideSupport::Required);
        assert_eq!(file.server, SideSupport::Unsupported);
    }

    #[test]
    fn parse_index_missing_env_defaults_to_required() {
        let index = json!({
            "formatVersion": 1,
            "name": "x",
            "files": [{ "path": "mods/a.jar", "downloads": ["u"] }],
            "dependencies": { "minecraft": "1.21", "quilt-loader": "0.1" }
        });
        let resolved = parse_index(&index).unwrap();
        assert_eq!(resolved.loader.as_deref(), Some("quilt"));
        assert_eq!(resolved.files[0].client, SideSupport::Required);
        assert_eq!(resolved.files[0].server, SideSupport::Required);
        assert!(resolved.files[0].artifact.checksum.is_none());
    }

    #[test]
    fn parse_index_rejects_bad_format() {
        let index = json!({ "formatVersion": 2, "dependencies": { "minecraft": "1.21" } });
        assert!(parse_index(&index).is_err());
    }

    #[test]
    fn parse_index_requires_minecraft() {
        let index = json!({ "formatVersion": 1, "dependencies": { "fabric-loader": "0.1" } });
        assert!(parse_index(&index).is_err());
    }

    #[test]
    fn parse_index_rejects_unsafe_paths() {
        for bad in ["../evil", "/etc/passwd", "mods/../../escape"] {
            let index = json!({
                "formatVersion": 1,
                "files": [{ "path": bad, "downloads": ["u"] }],
                "dependencies": { "minecraft": "1.21" }
            });
            assert!(parse_index(&index).is_err(), "should reject {bad}");
        }
    }

    #[test]
    fn facets_include_only_set_filters() {
        let query = SearchQuery {
            kind: ContentKind::Mod,
            loader: Some("fabric".into()),
            game_version: Some("1.21.1".into()),
            categories: vec!["optimization".into()],
            ..SearchQuery::default()
        };
        assert_eq!(
            build_facets(&query),
            r#"[["project_type:mod"],["categories:fabric"],["versions:1.21.1"],["categories:optimization"]]"#
        );

        let bare = SearchQuery {
            kind: ContentKind::Modpack,
            ..SearchQuery::default()
        };
        assert_eq!(build_facets(&bare), r#"[["project_type:modpack"]]"#);
    }

    #[test]
    fn sort_maps_to_modrinth_index() {
        assert_eq!(sort_index(SearchSort::Relevance), "relevance");
        assert_eq!(sort_index(SearchSort::Downloads), "downloads");
        assert_eq!(sort_index(SearchSort::Follows), "follows");
        assert_eq!(sort_index(SearchSort::Newest), "newest");
        assert_eq!(sort_index(SearchSort::Updated), "updated");
    }

    #[test]
    fn site_urls_parse_to_project_refs() {
        let m = Modrinth;
        for url in [
            "https://modrinth.com/mod/sodium",
            "http://modrinth.com/mod/sodium",
            "https://www.modrinth.com/mod/sodium/",
            "https://modrinth.com/mod/sodium?query=x#gallery",
            "https://modrinth.com/mod/sodium/versions",
        ] {
            let parsed = m
                .parse_url(url)
                .unwrap_or_else(|| panic!("should parse {url}"));
            assert_eq!(parsed.project, "sodium");
            assert_eq!(parsed.version, None, "{url}");
        }
        let pinned = m
            .parse_url("https://modrinth.com/mod/sodium/version/mc1.21.1-0.8.12-fabric")
            .unwrap();
        assert_eq!(pinned.version.as_deref(), Some("mc1.21.1-0.8.12-fabric"));

        assert!(m.parse_url("https://example.com/mod/sodium").is_none());
        assert!(m
            .parse_url("https://modrinth.com/user/jellysquid3")
            .is_none());
        assert!(m.parse_url("modrinth.com/mod/sodium").is_none());
        assert!(m.parse_url("https://modrinth.com/mod").is_none());
    }

    #[test]
    fn channel_and_dependency_strings_map() {
        assert_eq!(parse_channel("beta"), ReleaseChannel::Beta);
        assert_eq!(parse_channel("alpha"), ReleaseChannel::Alpha);
        assert_eq!(parse_channel("release"), ReleaseChannel::Release);
        assert_eq!(parse_channel("weird"), ReleaseChannel::Release);

        assert_eq!(parse_dependency_kind("optional"), DependencyKind::Optional);
        assert_eq!(
            parse_dependency_kind("incompatible"),
            DependencyKind::Incompatible
        );
        assert_eq!(parse_dependency_kind("embedded"), DependencyKind::Embedded);
        assert_eq!(parse_dependency_kind("required"), DependencyKind::Required);
        assert_eq!(parse_dependency_kind("unknown"), DependencyKind::Required);
    }
}
