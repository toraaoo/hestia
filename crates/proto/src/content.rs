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
#[serde(default, rename_all = "camelCase")]
pub struct ContentSource {
    pub id: String,
    pub name: String,
}

/// A gallery image on a project. Search hits carry only `url`; the detail call
/// fills the caption fields.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct GalleryImage {
    pub url: String,
    pub featured: bool,
    pub title: String,
    pub description: String,
}

/// A project, as a search hit or a detail. `body` (the long description) is only
/// filled by the detail call; `icon_url`/`gallery` carry images for the desktop UI.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
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
#[serde(default, rename_all = "camelCase")]
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
#[serde(default, rename_all = "camelCase")]
pub struct ContentDependency {
    pub project_id: String,
    pub version_id: String,
    pub kind: DependencyKind,
}

/// A downloadable version of a project.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
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
#[serde(default, rename_all = "camelCase")]
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
#[serde(default, rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<ContentProject>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
}

/// The versions of a project, optionally filtered by loader and game version.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct VersionQuery {
    pub source: String,
    pub project: String,
    pub loader: Option<String>,
    pub game_version: Option<String>,
}

/// One file a modpack pulls in, at its path relative to the game directory.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ModpackFile {
    pub path: String,
    pub artifact: Artifact,
    pub client: SideSupport,
    pub server: SideSupport,
}

/// A resolved modpack: the loader/game version it targets and the files to
/// place. `overrides/` handling is a materialize-time concern, deferred.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
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
#[serde(default, rename_all = "camelCase")]
pub struct SourcesResult {
    pub sources: Vec<ContentSource>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectParams {
    pub source: String,
    pub project: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct VersionsResult {
    pub versions: Vec<ContentVersion>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ModpackParams {
    pub source: String,
    pub version_id: String,
}

/// One installed content item, as the entry's index records it. `source` is a
/// platform id (`modrinth`) or the literal `file` for a local import — imports
/// carry empty project/version ids and cannot be updated.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
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
    /// The project's icon, carried for the desktop UI so an installed item
    /// renders its artwork; empty for local-file imports (no project) and for
    /// records written before this field.
    pub icon_url: String,
    pub installed_unix: i64,
    /// For datapacks: the world directory (relative to the entry's `data/`)
    /// the file lives in — datapacks load from inside a world, not a flat dir.
    /// Empty for every other kind.
    pub world: String,
    /// Who put the item in the pool: empty = user-installed; a global profile
    /// apply tags its installs `profile:<name>`.
    pub origin: String,
    /// Whether the launch-time mirror installs this item into `data/`. A
    /// disabled item keeps its managed copy and provenance but is not loaded by
    /// the game (for a datapack, its in-world file is renamed `.disabled`).
    /// Defaults to `true` so records written before this field decode enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// The installed items of one kind, plus filenames found in the entry's game
/// directory that no index entry accounts for.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentListResult {
    pub items: Vec<InstalledContent>,
    pub untracked: Vec<String>,
}

/// One thing to install: exactly one of `project` (a platform project,
/// optionally pinned by `version`), `url` (a project/version page URL on a
/// supported source), or `path` (a daemon-local file to import).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentAddItem {
    pub project: String,
    pub version: String,
    pub url: String,
    pub path: String,
    pub filename: String,
}

/// What to install: one or more items of one `kind` from one `source`,
/// installed in a single job. Items that fail are reported per item on the
/// done event; the rest of the batch proceeds.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentAddSpec {
    pub kind: ContentKind,
    pub source: String,
    pub items: Vec<ContentAddItem>,
    /// For datapacks on an instance: the save worlds each item installs into
    /// (the game loads datapacks from inside a world). Ignored for other
    /// kinds; a server uses its single `level-name` world.
    pub worlds: Vec<String>,
}

/// One item of a batch that could not be installed.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentFailure {
    /// The selector as given (project slug/id, URL, or path).
    pub item: String,
    /// The resolved project title, when resolution got that far.
    pub title: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentAddParams {
    pub server: String,
    #[serde(flatten)]
    pub spec: ContentAddSpec,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentAddParams {
    pub instance: String,
    #[serde(flatten)]
    pub spec: ContentAddSpec,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentListParams {
    pub server: String,
    pub kind: ContentKind,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentListParams {
    pub instance: String,
    pub kind: ContentKind,
}

/// `worlds` narrows a datapack removal to those save worlds (empty removes
/// every copy); it is rejected for the other kinds, which have no worlds.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentRemoveParams {
    pub server: String,
    pub kind: ContentKind,
    pub item: String,
    pub worlds: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentRemoveParams {
    pub instance: String,
    pub kind: ContentKind,
    pub item: String,
    pub worlds: Vec<String>,
}

/// `item` empty updates every platform-sourced item of the kind.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentUpdateParams {
    pub server: String,
    pub kind: ContentKind,
    pub item: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentUpdateParams {
    pub instance: String,
    pub kind: ContentKind,
    pub item: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentJobResult {
    pub id: String,
}

/// Enable or disable one installed item (matched by project id, slug, filename,
/// or title). Disabling drops it from the game's load dir while keeping the
/// managed copy and provenance; enabling restores the mirror. `worlds` narrows
/// a datapack toggle to those save worlds (empty toggles every copy).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentEnableParams {
    pub server: String,
    pub kind: ContentKind,
    pub item: String,
    pub enabled: bool,
    pub worlds: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentEnableParams {
    pub instance: String,
    pub kind: ContentKind,
    pub item: String,
    pub enabled: bool,
    pub worlds: Vec<String>,
}

/// Ask whether each platform-sourced item of the kind has a newer compatible
/// version. Resolves upstream, so it is a plain (network-bound) call rather
/// than an install job.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentCheckUpdatesParams {
    pub server: String,
    pub kind: ContentKind,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentCheckUpdatesParams {
    pub instance: String,
    pub kind: ContentKind,
}

/// One installed item's update status: its current pin against the newest
/// compatible version resolved upstream.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentUpdate {
    pub filename: String,
    pub project_id: String,
    /// For a datapack: the world the copy lives in (disambiguates one project
    /// installed into several worlds). Empty for the other kinds.
    pub world: String,
    pub current_version_id: String,
    pub current_version_number: String,
    pub latest_version_id: String,
    pub latest_version_number: String,
    /// True when the newest compatible version differs from the current pin.
    pub updatable: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ContentUpdatesResult {
    pub updates: Vec<ContentUpdate>,
}

/// Re-pin one installed item to a specific published `version` (id or number).
/// The swap re-installs that version like an update, applying at the next
/// start/launch; runs as a content job.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerContentSetVersionParams {
    pub server: String,
    pub kind: ContentKind,
    pub item: String,
    pub version: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct InstanceContentSetVersionParams {
    pub instance: String,
    pub kind: ContentKind,
    pub item: String,
    pub version: String,
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

pub struct ServerContentEnable;
impl Contract for ServerContentEnable {
    const CHANNEL: &'static str = "server.content.enable";
    type Params = ServerContentEnableParams;
    type Result = Empty;
}

pub struct InstanceContentEnable;
impl Contract for InstanceContentEnable {
    const CHANNEL: &'static str = "instance.content.enable";
    type Params = InstanceContentEnableParams;
    type Result = Empty;
}

pub struct ServerContentCheckUpdates;
impl Contract for ServerContentCheckUpdates {
    const CHANNEL: &'static str = "server.content.check_updates";
    type Params = ServerContentCheckUpdatesParams;
    type Result = ContentUpdatesResult;
}

pub struct InstanceContentCheckUpdates;
impl Contract for InstanceContentCheckUpdates {
    const CHANNEL: &'static str = "instance.content.check_updates";
    type Params = InstanceContentCheckUpdatesParams;
    type Result = ContentUpdatesResult;
}

pub struct ServerContentSetVersion;
impl Contract for ServerContentSetVersion {
    const CHANNEL: &'static str = "server.content.set_version";
    type Params = ServerContentSetVersionParams;
    type Result = ContentJobResult;
}

pub struct InstanceContentSetVersion;
impl Contract for InstanceContentSetVersion {
    const CHANNEL: &'static str = "instance.content.set_version";
    type Params = InstanceContentSetVersionParams;
    type Result = ContentJobResult;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentProgressEvent {
    pub id: String,
    #[serde(flatten)]
    pub progress: ProvisionProgress,
}
impl Topic for ContentProgressEvent {
    const TOPIC: &'static str = "content.progress";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentDoneEvent {
    pub id: String,
    pub items: Vec<InstalledContent>,
    #[serde(default)]
    pub failures: Vec<ContentFailure>,
}
impl Topic for ContentDoneEvent {
    const TOPIC: &'static str = "content.done";
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
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
