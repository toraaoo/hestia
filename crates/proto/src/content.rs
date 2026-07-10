//! Third-party content vocabulary shared by both sides of the socket. A *source*
//! is a platform (modrinth, curseforge, …); a source lists *projects* (mods,
//! modpacks, resourcepacks, shaders), each with its downloadable *versions*, and
//! resolves a modpack version into the file manifest a launcher installs. The
//! types are normalized so a front-end never sees a platform's raw shape, and all
//! carry `#[serde(default)]` so an older/newer peer decodes additive fields.

use serde::{Deserialize, Serialize};

use crate::contract::{Contract, Empty, Topic};
use crate::minecraft::{Artifact, ProvisionProgress};

/// What a project is — the second selector level after the source.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    #[default]
    Mod,
    Modpack,
    ResourcePack,
    Shader,
    DataPack,
}

/// Whether a project (or a modpack file) is meant for the client, the server,
/// both, or neither. `Unknown` when the source does not say.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SideSupport {
    Required,
    Optional,
    Unsupported,
    #[default]
    Unknown,
}

/// A version's release stability.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseChannel {
    #[default]
    Release,
    Beta,
    Alpha,
}

/// A source platform (modrinth, curseforge) — the first selector level.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentSource {
    pub id: String,
    pub name: String,
}

/// A gallery image on a project. Search hits carry only `url`; the detail call
/// fills the caption fields.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct GalleryImage {
    pub url: String,
    pub featured: bool,
    pub title: String,
    pub description: String,
}

/// A project, as a search hit or a detail. `body` (the long description) is only
/// filled by the detail call; `icon_url`/`gallery` carry images for the desktop UI.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentProject {
    pub source: String,
    pub id: String,
    pub slug: String,
    pub kind: ContentKind,
    pub title: String,
    pub description: String,
    pub body: String,
    pub author: String,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub follows: u64,
    pub icon_url: String,
    pub gallery: Vec<GalleryImage>,
    pub client_side: SideSupport,
    pub server_side: SideSupport,
}

/// One downloadable file of a version; `primary` marks the main artifact.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentFile {
    pub artifact: Artifact,
    pub primary: bool,
}

/// How a dependency relates to the version that declares it.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    #[default]
    Required,
    Optional,
    Incompatible,
    Embedded,
}

/// A dependency on another project (and optionally a specific version of it).
/// Either id may be empty when the source pins only the other.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentDependency {
    pub project_id: String,
    pub version_id: String,
    pub kind: DependencyKind,
}

/// A downloadable version of a project.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentVersion {
    pub source: String,
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub channel: ReleaseChannel,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub featured: bool,
    pub date_published: String,
    pub downloads: u64,
    pub files: Vec<ContentFile>,
    pub dependencies: Vec<ContentDependency>,
}

/// How search results are ordered.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

/// A paginated search over a source. `source` empty selects the default source;
/// `limit` is clamped to `1..=100` by the provider.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SearchQuery {
    pub source: String,
    pub kind: ContentKind,
    pub query: String,
    pub loader: Option<String>,
    pub game_version: Option<String>,
    pub categories: Vec<String>,
    pub sort: SearchSort,
    pub limit: u32,
    pub offset: u32,
}

/// A page of search hits. `total` is the full match count for paging.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SearchResult {
    pub hits: Vec<ContentProject>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
}

/// The versions of a project, optionally filtered by loader and game version.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct VersionQuery {
    pub source: String,
    pub project: String,
    pub loader: Option<String>,
    pub game_version: Option<String>,
}

/// One file a modpack pulls in, at its path relative to the game directory.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ModpackFile {
    pub path: String,
    pub artifact: Artifact,
    pub client: SideSupport,
    pub server: SideSupport,
}

/// A resolved modpack: the loader/game version it targets and the files to
/// place. `overrides/` handling is a materialize-time concern, deferred.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ResolvedModpack {
    pub source: String,
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub game_version: String,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub files: Vec<ModpackFile>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct SourcesResult {
    pub sources: Vec<ContentSource>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProjectParams {
    pub source: String,
    pub project: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct VersionsResult {
    pub versions: Vec<ContentVersion>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ModpackParams {
    pub source: String,
    pub version_id: String,
}

/// One installed content item, as the entry's index records it. `source` is a
/// platform id (`modrinth`) or the literal `file` for a local import — imports
/// carry empty project/version ids and cannot be updated.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstalledContent {
    pub kind: ContentKind,
    pub source: String,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub version_id: String,
    pub version_number: String,
    pub filename: String,
    pub sha1: String,
    pub url: String,
    pub installed_unix: i64,
    /// For datapacks: the world directory (relative to the entry's `data/`)
    /// the file lives in — datapacks load from inside a world, not a flat dir.
    /// Empty for every other kind.
    pub world: String,
}

/// The installed items of one kind, plus filenames found in the entry's game
/// directory that no index entry accounts for.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentListResult {
    pub items: Vec<InstalledContent>,
    pub untracked: Vec<String>,
}

/// What to install: exactly one of `project` (a platform project, optionally
/// pinned by `version`), `url` (a project/version page URL on a supported
/// source), or `path` (a daemon-local file to import).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentAddSpec {
    pub kind: ContentKind,
    pub source: String,
    pub project: String,
    pub version: String,
    pub url: String,
    pub path: String,
    pub filename: String,
    /// For a datapack on an instance: the save world to install into (the game
    /// loads datapacks from inside a world). Ignored for other kinds; a server
    /// uses its single `level-name` world.
    pub world: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerContentAddParams {
    pub server: String,
    #[serde(flatten)]
    pub spec: ContentAddSpec,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceContentAddParams {
    pub instance: String,
    #[serde(flatten)]
    pub spec: ContentAddSpec,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerContentListParams {
    pub server: String,
    pub kind: ContentKind,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceContentListParams {
    pub instance: String,
    pub kind: ContentKind,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerContentRemoveParams {
    pub server: String,
    pub kind: ContentKind,
    pub item: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceContentRemoveParams {
    pub instance: String,
    pub kind: ContentKind,
    pub item: String,
}

/// `item` empty updates every platform-sourced item of the kind.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ServerContentUpdateParams {
    pub server: String,
    pub kind: ContentKind,
    pub item: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceContentUpdateParams {
    pub instance: String,
    pub kind: ContentKind,
    pub item: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ContentJobResult {
    pub id: String,
}

pub struct ServerContentAdd;
impl Contract for ServerContentAdd {
    const CHANNEL: &'static str = "server.content.add";
    type Params = ServerContentAddParams;
    type Result = ContentJobResult;
}

pub struct ServerContentList;
impl Contract for ServerContentList {
    const CHANNEL: &'static str = "server.content.list";
    type Params = ServerContentListParams;
    type Result = ContentListResult;
}

pub struct ServerContentRemove;
impl Contract for ServerContentRemove {
    const CHANNEL: &'static str = "server.content.remove";
    type Params = ServerContentRemoveParams;
    type Result = Empty;
}

pub struct ServerContentUpdate;
impl Contract for ServerContentUpdate {
    const CHANNEL: &'static str = "server.content.update";
    type Params = ServerContentUpdateParams;
    type Result = ContentJobResult;
}

pub struct InstanceContentAdd;
impl Contract for InstanceContentAdd {
    const CHANNEL: &'static str = "instance.content.add";
    type Params = InstanceContentAddParams;
    type Result = ContentJobResult;
}

pub struct InstanceContentList;
impl Contract for InstanceContentList {
    const CHANNEL: &'static str = "instance.content.list";
    type Params = InstanceContentListParams;
    type Result = ContentListResult;
}

pub struct InstanceContentRemove;
impl Contract for InstanceContentRemove {
    const CHANNEL: &'static str = "instance.content.remove";
    type Params = InstanceContentRemoveParams;
    type Result = Empty;
}

pub struct InstanceContentUpdate;
impl Contract for InstanceContentUpdate {
    const CHANNEL: &'static str = "instance.content.update";
    type Params = InstanceContentUpdateParams;
    type Result = ContentJobResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: ProvisionProgress,
}
impl Topic for ContentProgressEvent {
    const TOPIC: &'static str = "content.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentDoneEvent {
    pub id: String,
    pub items: Vec<InstalledContent>,
}
impl Topic for ContentDoneEvent {
    const TOPIC: &'static str = "content.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentErrorEvent {
    pub id: String,
    pub message: String,
}
impl Topic for ContentErrorEvent {
    const TOPIC: &'static str = "content.error";
}

pub struct ContentSources;
impl Contract for ContentSources {
    const CHANNEL: &'static str = "content.sources";
    type Params = Empty;
    type Result = SourcesResult;
}

pub struct ContentSearch;
impl Contract for ContentSearch {
    const CHANNEL: &'static str = "content.search";
    type Params = SearchQuery;
    type Result = SearchResult;
}

pub struct ContentProjectGet;
impl Contract for ContentProjectGet {
    const CHANNEL: &'static str = "content.project";
    type Params = ProjectParams;
    type Result = ContentProject;
}

pub struct ContentVersions;
impl Contract for ContentVersions {
    const CHANNEL: &'static str = "content.versions";
    type Params = VersionQuery;
    type Result = VersionsResult;
}

pub struct ModpackResolve;
impl Contract for ModpackResolve {
    const CHANNEL: &'static str = "content.modpack.resolve";
    type Params = ModpackParams;
    type Result = ResolvedModpack;
}
