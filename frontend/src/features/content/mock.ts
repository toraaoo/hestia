/**
 * Static stand-in Modrinth projects, shaped after `content.search` /
 * `content.project`. Nothing talks to a backend.
 */

import type { ContentKind } from '@/lib/mock';

const DAY = 86_400;
const nowSec = Math.floor(Date.now() / 1000);
const daysAgo = (days: number) => nowSec - days * DAY;

/** A Modrinth project, from `content.search` / `content.project`. */
export interface ContentProject {
  id: string;
  title: string;
  author: string;
  description: string;
  kind: ContentKind;
  downloads: number;
  follows: number;
  categories: string[];
  updatedUnix: number;
}

export const contentProjects: ContentProject[] = [
  {
    id: 'sodium',
    title: 'Sodium',
    author: 'jellysquid3',
    description:
      'A modern rendering engine that massively improves frame rates and reduces stuttering.',
    kind: 'mod',
    downloads: 42_100_000,
    follows: 78_400,
    categories: ['Optimization'],
    updatedUnix: daysAgo(5),
  },
  {
    id: 'iris',
    title: 'Iris Shaders',
    author: 'coderbot',
    description:
      'A modern shaders mod compatible with the OptiFine shader pack ecosystem.',
    kind: 'mod',
    downloads: 28_600_000,
    follows: 61_200,
    categories: ['Optimization'],
    updatedUnix: daysAgo(9),
  },
  {
    id: 'fabric-api',
    title: 'Fabric API',
    author: 'modmuss50',
    description:
      'The essential hooks and interoperability mod for the Fabric toolchain.',
    kind: 'mod',
    downloads: 98_300_000,
    follows: 40_900,
    categories: ['Library'],
    updatedUnix: daysAgo(2),
  },
  {
    id: 'complementary',
    title: 'Complementary Shaders',
    author: 'EminGT',
    description:
      'A high-quality, well-optimized shader pack with a distinctive look.',
    kind: 'shader',
    downloads: 12_800_000,
    follows: 33_500,
    categories: ['Fantasy', 'Vanilla-like'],
    updatedUnix: daysAgo(15),
  },
  {
    id: 'lithium',
    title: 'Lithium',
    author: 'jellysquid3',
    description:
      'A general-purpose optimization mod for the game logic and tick loop.',
    kind: 'mod',
    downloads: 31_500_000,
    follows: 44_100,
    categories: ['Optimization'],
    updatedUnix: daysAgo(7),
  },
  {
    id: 'create',
    title: 'Create',
    author: 'simibubi',
    description: 'Aesthetic automation and contraptions with rotational power.',
    kind: 'mod',
    downloads: 21_200_000,
    follows: 52_300,
    categories: ['Technology'],
    updatedUnix: daysAgo(30),
  },
  {
    id: 'faithful',
    title: 'Faithful 32x',
    author: 'Faithful Team',
    description:
      'A faithful, higher-resolution take on the default texture pack.',
    kind: 'resourcepack',
    downloads: 9_400_000,
    follows: 18_700,
    categories: ['Vanilla-like'],
    updatedUnix: daysAgo(22),
  },
  {
    id: 'voice-chat',
    title: 'Simple Voice Chat',
    author: 'henkelmax',
    description:
      'Proximity voice chat for any server, no extra software required.',
    kind: 'mod',
    downloads: 17_900_000,
    follows: 29_800,
    categories: ['Social'],
    updatedUnix: daysAgo(12),
  },
];

export const getProject = (id: string) =>
  contentProjects.find((p) => p.id === id);

/** A downloadable version of a project, from `content.versions`. */
export interface ContentVersion {
  id: string;
  versionNumber: string;
  channel: 'release' | 'beta' | 'alpha';
  gameVersions: string[];
  loaders: string[];
  publishedUnix: number;
  downloads: number;
  filename: string;
}

/** Game versions a project's builds target, newest first. */
const VERSION_GAMES = ['1.21.4', '1.21.3', '1.21.1', '1.21', '1.20.6'];

/** The loaders a kind's files declare — mods pin a loader, the rest don't. */
function kindLoaders(kind: ContentKind): string[] {
  switch (kind) {
    case 'mod':
    case 'modpack':
      return ['fabric'];
    case 'datapack':
      return ['datapack'];
    case 'resourcepack':
      return ['minecraft'];
    case 'shader':
      return ['iris', 'optifine'];
  }
}

/**
 * A deterministic version list for a project, newest first — the stand-in for
 * `content.versions`. The three builds descend in version and reach back one
 * more game version each, so older builds cover a wider compatibility range.
 */
export function projectVersions(project: ContentProject): ContentVersion[] {
  const loaders = kindLoaders(project.kind);
  return [0, 1, 2].map((seq) => ({
    id: `${project.id}-v${seq}`,
    versionNumber: `1.${5 - seq}.${(project.id.length + seq) % 9}`,
    channel: seq === 2 ? 'beta' : 'release',
    gameVersions: VERSION_GAMES.slice(0, 3 + seq),
    loaders,
    publishedUnix: project.updatedUnix - seq * 18 * DAY,
    downloads: Math.round(project.downloads / (seq + 3)),
    filename: `${project.id}-1.${5 - seq}.${(project.id.length + seq) % 9}.jar`,
  }));
}

/** Required-dependency edges — one project pulls another in on install. */
const REQUIRES: Record<string, string[]> = {
  create: ['fabric-api'],
  lithium: ['fabric-api'],
  iris: ['sodium'],
  'voice-chat': ['fabric-api'],
};

/**
 * Resolves a project's required dependencies breadth-first, mirroring the
 * daemon's install-time resolve: transitive, de-duplicated, and never
 * including the project itself.
 */
export function resolveDependencies(projectId: string): ContentProject[] {
  const seen = new Set<string>([projectId]);
  const out: ContentProject[] = [];
  const queue = [...(REQUIRES[projectId] ?? [])];
  while (queue.length > 0) {
    const id = queue.shift();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    const dep = getProject(id);
    if (dep) out.push(dep);
    queue.push(...(REQUIRES[id] ?? []));
  }
  return out;
}
