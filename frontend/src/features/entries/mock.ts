/**
 * Static stand-in instances and servers, shaped after the daemon's real
 * surface (`server.list` / `instance.list` and their sub-resources) so the
 * pages mirror actual functionality. Nothing talks to a backend.
 */

import type { ContentKind } from '@/lib/mock';

/** An installed piece of content, from `content.list`. */
export interface InstalledContent {
  id: string;
  name: string;
  kind: ContentKind;
  source: string;
  version: string;
  enabled: boolean;
  updatable: boolean;
}

/** A backup archive, from `backup.list`. */
export interface Backup {
  id: string;
  created_unix: number;
  kind: 'manual' | 'scheduled' | 'update';
  size_bytes: number;
}

const DAY = 86_400;
const nowSec = Math.floor(Date.now() / 1000);
const daysAgo = (days: number) => nowSec - days * DAY;
const hoursAgo = (hours: number) => nowSec - hours * 3600;

export interface Instance {
  id: string;
  name: string;
  flavor: string;
  game_version: string;
  loader_version?: string;
  java_major: number;
  memory: string;
  created_unix: number;
  last_played_unix: number;
  running: boolean;
  sessions: number;
  cpu_pct: number;
  mem_used_mb: number;
  disk_bytes: number;
  content: InstalledContent[];
  worlds: string[];
}

export const instances: Instance[] = [
  {
    id: 'aether-skies-3f9a2c7d',
    name: 'Aether Skies',
    flavor: 'fabric',
    game_version: '1.21.4',
    loader_version: '0.16.9',
    java_major: 21,
    memory: '6G',
    created_unix: daysAgo(96),
    last_played_unix: hoursAgo(2),
    running: true,
    sessions: 1,
    cpu_pct: 47,
    mem_used_mb: 4280,
    disk_bytes: 3_200_000_000,
    content: [
      {
        id: 'sodium',
        name: 'Sodium',
        kind: 'mod',
        source: 'modrinth',
        version: '0.6.13',
        enabled: true,
        updatable: true,
      },
      {
        id: 'iris',
        name: 'Iris Shaders',
        kind: 'mod',
        source: 'modrinth',
        version: '1.8.11',
        enabled: true,
        updatable: false,
      },
      {
        id: 'fabric-api',
        name: 'Fabric API',
        kind: 'mod',
        source: 'modrinth',
        version: '0.116.4',
        enabled: true,
        updatable: false,
      },
      {
        id: 'complementary',
        name: 'Complementary Shaders',
        kind: 'shader',
        source: 'modrinth',
        version: 'r5.5',
        enabled: true,
        updatable: false,
      },
      {
        id: 'faithful',
        name: 'Faithful 32x',
        kind: 'resourcepack',
        source: 'modrinth',
        version: '1.21',
        enabled: false,
        updatable: false,
      },
    ],
    worlds: ['New World', 'Skyblock Run', 'Creative Flats'],
  },
  {
    id: 'vanilla-1214-a1b2c3d4',
    name: 'Vanilla 1.21',
    flavor: 'vanilla',
    game_version: '1.21.4',
    java_major: 21,
    memory: '4G',
    created_unix: daysAgo(120),
    last_played_unix: daysAgo(4),
    running: false,
    sessions: 0,
    cpu_pct: 0,
    mem_used_mb: 0,
    disk_bytes: 640_000_000,
    content: [],
    worlds: ['Survival'],
  },
  {
    id: 'create-above-77ffee11',
    name: 'Create: Above & Beyond',
    flavor: 'fabric',
    game_version: '1.20.1',
    loader_version: '0.15.11',
    java_major: 17,
    memory: '8G',
    created_unix: daysAgo(210),
    last_played_unix: daysAgo(23),
    running: false,
    sessions: 0,
    cpu_pct: 0,
    mem_used_mb: 0,
    disk_bytes: 2_800_000_000,
    content: [
      {
        id: 'create',
        name: 'Create',
        kind: 'mod',
        source: 'modrinth',
        version: '0.5.1',
        enabled: true,
        updatable: true,
      },
      {
        id: 'jei',
        name: 'Just Enough Items',
        kind: 'mod',
        source: 'modrinth',
        version: '15.3.0',
        enabled: true,
        updatable: false,
      },
    ],
    worlds: ['Factory'],
  },
  {
    id: 'snapshot-lab-9090abab',
    name: 'Snapshot Lab',
    flavor: 'vanilla',
    game_version: '25w03a',
    java_major: 21,
    memory: '4G',
    created_unix: daysAgo(150),
    last_played_unix: daysAgo(58),
    running: false,
    sessions: 0,
    cpu_pct: 0,
    mem_used_mb: 0,
    disk_bytes: 120_000_000,
    content: [],
    worlds: [],
  },
];

/** Instances the user pinned for quick access (the play-bar picker). */
export const pinnedInstanceIds = [
  'aether-skies-3f9a2c7d',
  'create-above-77ffee11',
];
export const pinnedInstances = instances.filter((i) =>
  pinnedInstanceIds.includes(i.id),
);

export interface Server {
  id: string;
  name: string;
  flavor: string;
  game_version: string;
  loader_version?: string;
  java_major: number;
  memory: string;
  created_unix: number;
  ready: boolean;
  running: boolean;
  cpu_pct: number;
  mem_used_mb: number;
  disk_bytes: number;
  port?: number;
  rcon_port?: number;
  players: number;
  max_players: number;
  motd: string;
  backup_interval: string;
  backup_retention: number;
  content: InstalledContent[];
  backups: Backup[];
}

export const servers: Server[] = [
  {
    id: 'smp-cottage-3f9a2c7d',
    name: 'Cottage SMP',
    flavor: 'vanilla',
    game_version: '1.21.4',
    java_major: 21,
    memory: '4G',
    created_unix: daysAgo(88),
    ready: true,
    running: true,
    cpu_pct: 34,
    mem_used_mb: 2610,
    disk_bytes: 890_000_000,
    port: 25565,
    rcon_port: 25575,
    players: 3,
    max_players: 20,
    motd: 'A cozy survival server',
    backup_interval: '6h',
    backup_retention: 10,
    content: [
      {
        id: 'vanilla-tweaks',
        name: 'VanillaTweaks',
        kind: 'datapack',
        source: 'file',
        version: '—',
        enabled: true,
        updatable: false,
      },
    ],
    backups: [
      {
        id: 's1',
        created_unix: hoursAgo(1),
        kind: 'scheduled',
        size_bytes: 96_000_000,
      },
      {
        id: 's2',
        created_unix: hoursAgo(7),
        kind: 'scheduled',
        size_bytes: 95_200_000,
      },
      {
        id: 's3',
        created_unix: daysAgo(1),
        kind: 'manual',
        size_bytes: 94_800_000,
      },
    ],
  },
  {
    id: 'modded-hub-1122ccdd',
    name: 'Modded Hub',
    flavor: 'fabric',
    game_version: '1.20.1',
    loader_version: '0.15.11',
    java_major: 17,
    memory: '6G',
    created_unix: daysAgo(60),
    ready: true,
    running: false,
    cpu_pct: 0,
    mem_used_mb: 0,
    disk_bytes: 1_400_000_000,
    port: 25566,
    players: 0,
    max_players: 40,
    motd: 'Modded chaos',
    backup_interval: '',
    backup_retention: 5,
    content: [
      {
        id: 'lithium',
        name: 'Lithium',
        kind: 'mod',
        source: 'modrinth',
        version: '0.11.2',
        enabled: true,
        updatable: false,
      },
      {
        id: 'fabric-api',
        name: 'Fabric API',
        kind: 'mod',
        source: 'modrinth',
        version: '0.92.0',
        enabled: true,
        updatable: true,
      },
    ],
    backups: [],
  },
  {
    id: 'hardcore-9f9f9f9f',
    name: 'Hardcore Trials',
    flavor: 'vanilla',
    game_version: '1.21.1',
    java_major: 21,
    memory: '4G',
    created_unix: daysAgo(14),
    ready: false,
    running: false,
    cpu_pct: 0,
    mem_used_mb: 0,
    disk_bytes: 210_000_000,
    players: 0,
    max_players: 10,
    motd: 'One life.',
    backup_interval: '12h',
    backup_retention: 3,
    content: [],
    backups: [],
  },
];

export const getInstance = (id: string) => instances.find((i) => i.id === id);
export const getServer = (id: string) => servers.find((s) => s.id === id);

/** The live-metric slice of an entry, keyed by id across both stores. */
export interface EntryResources {
  running: boolean;
  cpu_pct: number;
  mem_used_mb: number;
  memory: string;
  disk_bytes: number;
}

/**
 * Resolves an entry's resource usage by id — the mock stand-in for the daemon's
 * per-process metrics, which a server and an instance expose the same way.
 */
export function getEntryResources(id: string): EntryResources | undefined {
  const entry = getServer(id) ?? getInstance(id);
  if (!entry) return undefined;
  const { running, cpu_pct, mem_used_mb, memory, disk_bytes } = entry;
  return { running, cpu_pct, mem_used_mb, memory, disk_bytes };
}

/** The world resumed by the library's "continue playing". */
export const featured = instances[0];
