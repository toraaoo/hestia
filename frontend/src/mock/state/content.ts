/**
 * Content: the platform catalogue a browse answers from, and the installed
 * pool each entry carries. The catalogue is fixed; the pools are the mutable
 * half — an add, an enable, a re-pin and a removal all land here, so the
 * content tab reflects what was just done to it.
 */
import type {
  ContentKind,
  ContentProject,
  ContentUpdate,
  ContentVersion,
  InstalledContent,
  ResolvedModpack,
  ResolvedUrl,
  SearchResult,
  UntrackedFile,
} from '@/api/types';

import { ago, fail, now } from '../support';

const ICON = (id: string) => `https://cdn.modrinth.com/data/${id}/icon.png`;

function project(
  partial: Pick<ContentProject, 'id' | 'slug' | 'kind' | 'title' | 'author'> &
    Partial<ContentProject>,
): ContentProject {
  return {
    source: 'modrinth',
    kinds: [partial.kind],
    description: `${partial.title} — a fixture project served by the browser mock.`,
    body: `## ${partial.title}\n\nRendered from the fixture daemon, not from a platform.`,
    categories: ['optimization'],
    downloads: 1_200_000,
    follows: 4_300,
    iconUrl: ICON(partial.id),
    gallery: [],
    clientSide: 'required',
    serverSide: 'optional',
    ...partial,
  };
}

const catalogue: ContentProject[] = [
  project({
    id: 'AANobbMI',
    slug: 'sodium',
    kind: 'mod',
    title: 'Sodium',
    author: 'jellysquid3',
    downloads: 42_000_000,
  }),
  project({
    id: 'P7dR8mSH',
    slug: 'fabric-api',
    kind: 'mod',
    title: 'Fabric API',
    author: 'modmuss50',
    categories: ['library'],
    downloads: 88_000_000,
    serverSide: 'required',
  }),
  project({
    id: 'gvQqBUqZ',
    slug: 'lithium',
    kind: 'mod',
    title: 'Lithium',
    author: 'jellysquid3',
    serverSide: 'required',
  }),
  project({
    id: 'YL57xq9U',
    slug: 'iris',
    kind: 'mod',
    title: 'Iris Shaders',
    author: 'coderbot',
    categories: ['optimization', 'utility'],
    serverSide: 'unsupported',
  }),
  project({
    id: 'HVnmMxH1',
    slug: 'complementary-reimagined',
    kind: 'shader',
    title: 'Complementary Shaders — Reimagined',
    author: 'EminGT',
    categories: ['realistic'],
    serverSide: 'unsupported',
  }),
  project({
    id: '1KVo5zza',
    slug: 'faithful-32x',
    kind: 'resource_pack',
    title: 'Faithful 32x',
    author: 'Faithful Team',
    categories: ['decoration'],
  }),
  project({
    id: '8oi3bsk5',
    slug: 'terralith',
    kind: 'data_pack',
    title: 'Terralith',
    author: 'Starmute',
    categories: ['worldgen'],
    serverSide: 'required',
  }),
  project({
    id: 'hZLYEnCH',
    slug: 'essentialsx',
    kind: 'plugin',
    title: 'EssentialsX',
    author: 'EssentialsX Team',
    categories: ['management'],
    clientSide: 'unsupported',
    serverSide: 'required',
  }),
  project({
    id: '1KVo5zzb',
    slug: 'fabulously-optimized',
    kind: 'modpack',
    title: 'Fabulously Optimized',
    author: 'Robotkoer',
    categories: ['optimization'],
  }),
];

export function search(
  kind: ContentKind | undefined,
  query: string,
  limit: number,
  offset: number,
): SearchResult {
  const needle = query.trim().toLowerCase();
  const matched = catalogue.filter(
    (entry) =>
      (!kind || entry.kind === kind) &&
      (needle === '' ||
        entry.title.toLowerCase().includes(needle) ||
        entry.slug.includes(needle)),
  );
  return {
    hits: matched.slice(offset, offset + limit),
    offset,
    limit,
    total: matched.length,
  };
}

export function find(ref: string): ContentProject {
  const found = catalogue.find(
    (entry) => entry.id === ref || entry.slug === ref,
  );
  if (!found) fail('not_found', `no such project: ${ref}`);
  return found;
}

const VERSION_NUMBERS = ['1.4.0', '1.3.2', '1.2.9'];

export function versionsOf(ref: string): ContentVersion[] {
  const found = find(ref);
  return VERSION_NUMBERS.map((number, index) => ({
    source: found.source,
    id: `${found.id}-v${index}`,
    projectId: found.id,
    name: `${found.title} ${number}`,
    versionNumber: number,
    channel: index === 0 ? 'release' : 'beta',
    gameVersions: ['1.21.4', '1.21.1'],
    loaders: ['fabric', 'neoforge'],
    featured: index === 0,
    datePublished: new Date((now() - index * 86_400 * 30) * 1000).toISOString(),
    downloads: 900_000 - index * 120_000,
    files: [
      {
        artifact: {
          url: `https://cdn.modrinth.com/data/${found.id}/versions/${number}/${found.slug}-${number}.jar`,
          filename: `${found.slug}-${number}.jar`,
          size: 412_000,
          checksum: { algorithm: 'sha1', hex: '0'.repeat(40) },
        },
        primary: true,
      },
    ],
    dependencies: [],
  }));
}

/** The project (and pinned version) a source page URL names. */
export function resolveUrl(url: string): ResolvedUrl {
  const [, slug = '', version = ''] =
    /\/(?:mod|modpack|resourcepack|shader|datapack|plugin)\/([^/?#]+)(?:\/version\/([^/?#]+))?/.exec(
      url,
    ) ?? [];
  const found = catalogue.find((entry) => entry.slug === slug) ?? catalogue[0];
  return { project: found, versionId: version };
}

export function resolveModpack(versionId: string): ResolvedModpack {
  const pack =
    catalogue.find((entry) => entry.kind === 'modpack') ?? catalogue[0];
  return {
    source: pack.source,
    projectId: pack.id,
    versionId: versionId || `${pack.id}-v0`,
    versionNumber: '1.4.0',
    name: pack.title,
    summary: pack.description,
    gameVersion: '1.21.1',
    loader: 'fabric',
    loaderVersion: '0.16.5',
    files: [],
  };
}

function installed(
  entry: ContentProject,
  kind: ContentKind,
  origin = '',
): InstalledContent {
  const version = versionsOf(entry.id)[0];
  return {
    kind,
    source: entry.source,
    projectId: entry.id,
    slug: entry.slug,
    title: entry.title,
    versionId: version.id,
    versionNumber: version.versionNumber,
    filename: version.files[0].artifact.filename,
    sha1: '0'.repeat(40),
    url: version.files[0].artifact.url,
    iconUrl: entry.iconUrl,
    installedUnix: ago(86_400 * 2),
    worlds: [],
    origin,
    enabled: true,
    disabledWorlds: [],
  };
}

const pools = new Map<string, InstalledContent[]>([
  [
    'fabric-playground',
    [
      installed(find('sodium'), 'mod'),
      installed(find('fabric-api'), 'mod'),
      installed(find('iris'), 'mod'),
      installed(find('complementary-reimagined'), 'shader'),
      installed(find('faithful-32x'), 'resource_pack'),
    ],
  ],
  ['smp', [installed(find('terralith'), 'data_pack')]],
  ['creative', [installed(find('essentialsx'), 'plugin')]],
]);

const untracked = new Map<string, UntrackedFile[]>([
  [
    'fabric-playground',
    [
      {
        name: 'hand-dropped.jar',
        path: '/mock/.hestia/instances/fabric-playground/data/mods/hand-dropped.jar',
      },
    ],
  ],
]);

const pool = (id: string): InstalledContent[] => {
  const existing = pools.get(id);
  if (existing) return existing;
  const fresh: InstalledContent[] = [];
  pools.set(id, fresh);
  return fresh;
};

export function listPool(
  id: string,
  kind: ContentKind,
): { items: InstalledContent[]; untracked: UntrackedFile[] } {
  return {
    items: pool(id).filter((item) => item.kind === kind),
    untracked: kind === 'mod' ? (untracked.get(id) ?? []) : [],
  };
}

/** Install one item per requested reference, ignoring ones already in the pool. */
export function install(
  id: string,
  kind: ContentKind,
  refs: string[],
  origin = '',
): InstalledContent[] {
  const items = pool(id);
  const added: InstalledContent[] = [];
  for (const ref of refs) {
    const entry = catalogue.find(
      (candidate) => candidate.id === ref || candidate.slug === ref,
    );
    if (!entry || items.some((item) => item.projectId === entry.id)) continue;
    const item = { ...installed(entry, kind, origin), installedUnix: now() };
    items.push(item);
    added.push(item);
  }
  return added;
}

/** Install a local file — no project, so it can never be updated. */
export function installFile(
  id: string,
  kind: ContentKind,
  path: string,
): InstalledContent {
  const filename = path.split(/[/\\]/).pop() ?? 'content.jar';
  const item: InstalledContent = {
    kind,
    source: 'file',
    projectId: '',
    slug: '',
    title: filename.replace(/\.(jar|zip)$/i, ''),
    versionId: '',
    versionNumber: '',
    filename,
    sha1: '0'.repeat(40),
    url: '',
    iconUrl: '',
    installedUnix: now(),
    worlds: [],
    origin: '',
    enabled: true,
    disabledWorlds: [],
  };
  pool(id).push(item);
  return item;
}

const isItem = (item: InstalledContent, ref: string): boolean =>
  item.filename === ref ||
  item.projectId === ref ||
  item.slug === ref ||
  item.title === ref;

export function remove(id: string, kind: ContentKind, ref: string): void {
  const items = pool(id);
  const at = items.findIndex((item) => item.kind === kind && isItem(item, ref));
  if (at >= 0) items.splice(at, 1);
}

export function setEnabled(
  id: string,
  kind: ContentKind,
  ref: string,
  enabled: boolean,
): void {
  const item = pool(id).find(
    (candidate) => candidate.kind === kind && isItem(candidate, ref),
  );
  if (item) item.enabled = enabled;
}

/** Re-pin an item; an unknown version number is taken at face value. */
export function setVersion(
  id: string,
  kind: ContentKind,
  ref: string,
  version: string,
): InstalledContent[] {
  const item = pool(id).find(
    (candidate) => candidate.kind === kind && isItem(candidate, ref),
  );
  if (!item) return [];
  const match = versionsOf(item.projectId).find(
    (candidate) =>
      candidate.id === version || candidate.versionNumber === version,
  );
  item.versionId = match?.id ?? version;
  item.versionNumber = match?.versionNumber ?? version;
  return [item];
}

/** Move every platform-sourced item of the kind (or one) to its newest pin. */
export function update(
  id: string,
  kind: ContentKind,
  ref: string,
): InstalledContent[] {
  const items = pool(id).filter(
    (item) =>
      item.kind === kind &&
      item.source !== 'file' &&
      (!ref || isItem(item, ref)),
  );
  for (const item of items) {
    const latest = versionsOf(item.projectId)[0];
    item.versionId = latest.id;
    item.versionNumber = latest.versionNumber;
  }
  return items;
}

/** The pool's update status — the second item of each kind is behind. */
export function updates(id: string, kind: ContentKind): ContentUpdate[] {
  return pool(id)
    .filter((item) => item.kind === kind && item.source !== 'file')
    .map((item, index) => {
      const latest = versionsOf(item.projectId)[0];
      return {
        filename: item.filename,
        projectId: item.projectId,
        currentVersionId: item.versionId,
        currentVersionNumber: item.versionNumber,
        latestVersionId: latest.id,
        latestVersionNumber: latest.versionNumber,
        updatable: index % 2 === 1,
      };
    });
}

/** Drop everything a pack installed, for `*.modpack.remove`. */
export function removeByOrigin(id: string, origin: string): number {
  const items = pool(id);
  const kept = items.filter((item) => item.origin !== origin);
  const removed = items.length - kept.length;
  pools.set(id, kept);
  return removed;
}

export const poolOf = (id: string): InstalledContent[] => pool(id);
