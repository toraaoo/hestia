/** Mirrors `crates/proto/src/content.rs`. */
import type { Artifact } from './minecraft';

export type ContentKind =
  | 'mod'
  | 'modpack'
  | 'resource_pack'
  | 'shader'
  | 'data_pack';

export type SideSupport = 'required' | 'optional' | 'unsupported' | 'unknown';

export type ReleaseChannel = 'release' | 'beta' | 'alpha';

export interface ContentSource {
  id: string;
  name: string;
}

export interface GalleryImage {
  url: string;
  featured: boolean;
  title: string;
  description: string;
}

/**
 * A project, as a search hit or a detail. `body` (the long description) is
 * only filled by the detail call.
 */
export interface ContentProject {
  source: string;
  id: string;
  slug: string;
  kind: ContentKind;
  title: string;
  description: string;
  body: string;
  author: string;
  categories: string[];
  downloads: number;
  follows: number;
  iconUrl: string;
  gallery: GalleryImage[];
  clientSide: SideSupport;
  serverSide: SideSupport;
}

export interface ContentFile {
  artifact: Artifact;
  primary: boolean;
}

export type DependencyKind =
  | 'required'
  | 'optional'
  | 'incompatible'
  | 'embedded';

export interface ContentDependency {
  projectId: string;
  versionId: string;
  kind: DependencyKind;
}

export interface ContentVersion {
  source: string;
  id: string;
  projectId: string;
  name: string;
  versionNumber: string;
  channel: ReleaseChannel;
  gameVersions: string[];
  loaders: string[];
  featured: boolean;
  datePublished: string;
  downloads: number;
  files: ContentFile[];
  dependencies: ContentDependency[];
}

export type SearchSort =
  | 'relevance'
  | 'downloads'
  | 'follows'
  | 'newest'
  | 'updated';

/** A paginated search; `source` empty selects the default source. */
export interface SearchQuery {
  source?: string;
  kind?: ContentKind;
  query?: string;
  loader?: string;
  gameVersion?: string;
  categories?: string[];
  sort?: SearchSort;
  limit?: number;
  offset?: number;
}

export interface SearchResult {
  hits: ContentProject[];
  offset: number;
  limit: number;
  total: number;
}

export interface VersionQuery {
  source?: string;
  project: string;
  loader?: string;
  gameVersion?: string;
}

export interface ModpackFile {
  path: string;
  artifact: Artifact;
  client: SideSupport;
  server: SideSupport;
}

export interface ResolvedModpack {
  source: string;
  projectId: string;
  versionId: string;
  name: string;
  gameVersion: string;
  loader?: string;
  loaderVersion?: string;
  files: ModpackFile[];
}

/**
 * One installed content item. `source` is a platform id (`modrinth`) or the
 * literal `file` for a local import — imports cannot be updated.
 */
export interface InstalledContent {
  kind: ContentKind;
  source: string;
  projectId: string;
  slug: string;
  title: string;
  versionId: string;
  versionNumber: string;
  filename: string;
  sha1: string;
  url: string;
  installedUnix: number;
  /** For datapacks: the world the file lives in; empty for other kinds. */
  world: string;
  /**
   * Who put the item in the pool: empty = user-installed; a global profile
   * apply tags its installs `profile:<name>`.
   */
  origin: string;
}

export interface ContentList {
  items: InstalledContent[];
  /** Filenames in the game dir that no index entry accounts for. */
  untracked: string[];
}

/**
 * One thing to install: exactly one of `project` (optionally pinned by
 * `version`), `url` (a source page URL), or `path` (a daemon-local file).
 */
export interface ContentAddItem {
  project?: string;
  version?: string;
  url?: string;
  path?: string;
  filename?: string;
}

export interface ContentAddSpec {
  kind: ContentKind;
  source?: string;
  items: ContentAddItem[];
  /** For datapacks on an instance: the save worlds each item installs into. */
  worlds?: string[];
}

export interface ContentFailure {
  item: string;
  title: string;
  message: string;
}

export interface ContentDone {
  id: string;
  items: InstalledContent[];
  failures: ContentFailure[];
}
