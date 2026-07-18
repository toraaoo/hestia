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
  /** The global profile that installed it (`instance.profile.apply`), if any. */
  origin?: string;
}

/**
 * A named selection over an instance's installed pool, from
 * `instance.profile.list`. Members reference content ids here (the wire keys
 * them by filename); `captured` mirrors the profile owning its own settings
 * store.
 */
export interface ContentProfile {
  name: string;
  members: string[];
  captured: boolean;
}

/** A backup archive, from `backup.list`. */
export interface Backup {
  id: string;
  createdUnix: number;
  kind: 'manual' | 'scheduled' | 'update';
  sizeBytes: number;
}

const DAY = 86_400;
const nowSec = Math.floor(Date.now() / 1000);
const daysAgo = (days: number) => nowSec - days * DAY;
const hoursAgo = (hours: number) => nowSec - hours * 3600;

export interface Instance {
  id: string;
  name: string;
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  javaMajor: number;
  memory: string;
  createdUnix: number;
  lastPlayedUnix: number;
  running: boolean;
  sessions: number;
  cpuPct: number;
  memUsedMb: number;
  diskBytes: number;
  content: InstalledContent[];
  worlds: string[];
  /** The active content profile's name; empty when none is active. */
  activeProfile: string;
  profiles: ContentProfile[];
}

export const instances: Instance[] = [
  {
    id: 'aether-skies-3f9a2c7d',
    name: 'Aether Skies',
    flavor: 'fabric',
    gameVersion: '1.21.4',
    loaderVersion: '0.16.9',
    javaMajor: 21,
    memory: '6G',
    createdUnix: daysAgo(96),
    lastPlayedUnix: hoursAgo(2),
    running: true,
    sessions: 1,
    cpuPct: 47,
    memUsedMb: 4280,
    diskBytes: 3_200_000_000,
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
    activeProfile: 'performance',
    profiles: [
      {
        name: 'performance',
        members: ['sodium', 'iris', 'fabric-api'],
        captured: false,
      },
      {
        name: 'showcase',
        members: ['sodium', 'iris', 'fabric-api', 'complementary', 'faithful'],
        captured: true,
      },
    ],
  },
  {
    id: 'vanilla-1214-a1b2c3d4',
    name: 'Vanilla 1.21',
    flavor: 'vanilla',
    gameVersion: '1.21.4',
    javaMajor: 21,
    memory: '4G',
    createdUnix: daysAgo(120),
    lastPlayedUnix: daysAgo(4),
    running: false,
    sessions: 0,
    cpuPct: 0,
    memUsedMb: 0,
    diskBytes: 640_000_000,
    content: [],
    worlds: ['Survival'],
    activeProfile: '',
    profiles: [],
  },
  {
    id: 'create-above-77ffee11',
    name: 'Create: Above & Beyond',
    flavor: 'fabric',
    gameVersion: '1.20.1',
    loaderVersion: '0.15.11',
    javaMajor: 17,
    memory: '8G',
    createdUnix: daysAgo(210),
    lastPlayedUnix: daysAgo(23),
    running: false,
    sessions: 0,
    cpuPct: 0,
    memUsedMb: 0,
    diskBytes: 2_800_000_000,
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
        origin: 'qol',
      },
    ],
    worlds: ['Factory'],
    activeProfile: '',
    profiles: [{ name: 'lightweight', members: ['create'], captured: false }],
  },
  {
    id: 'snapshot-lab-9090abab',
    name: 'Snapshot Lab',
    flavor: 'vanilla',
    gameVersion: '25w03a',
    javaMajor: 21,
    memory: '4G',
    createdUnix: daysAgo(150),
    lastPlayedUnix: daysAgo(58),
    running: false,
    sessions: 0,
    cpuPct: 0,
    memUsedMb: 0,
    diskBytes: 120_000_000,
    content: [],
    worlds: [],
    activeProfile: '',
    profiles: [],
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
  gameVersion: string;
  loaderVersion?: string;
  javaMajor: number;
  memory: string;
  createdUnix: number;
  ready: boolean;
  running: boolean;
  cpuPct: number;
  memUsedMb: number;
  diskBytes: number;
  port?: number;
  rconPort?: number;
  players: number;
  maxPlayers: number;
  motd: string;
  backupInterval: string;
  backupRetention: number;
  content: InstalledContent[];
  backups: Backup[];
}

export const servers: Server[] = [
  {
    id: 'smp-cottage-3f9a2c7d',
    name: 'Cottage SMP',
    flavor: 'vanilla',
    gameVersion: '1.21.4',
    javaMajor: 21,
    memory: '4G',
    createdUnix: daysAgo(88),
    ready: true,
    running: true,
    cpuPct: 34,
    memUsedMb: 2610,
    diskBytes: 890_000_000,
    port: 25565,
    rconPort: 25575,
    players: 3,
    maxPlayers: 20,
    motd: 'A cozy survival server',
    backupInterval: '6h',
    backupRetention: 10,
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
        createdUnix: hoursAgo(1),
        kind: 'scheduled',
        sizeBytes: 96_000_000,
      },
      {
        id: 's2',
        createdUnix: hoursAgo(7),
        kind: 'scheduled',
        sizeBytes: 95_200_000,
      },
      {
        id: 's3',
        createdUnix: daysAgo(1),
        kind: 'manual',
        sizeBytes: 94_800_000,
      },
    ],
  },
  {
    id: 'modded-hub-1122ccdd',
    name: 'Modded Hub',
    flavor: 'fabric',
    gameVersion: '1.20.1',
    loaderVersion: '0.15.11',
    javaMajor: 17,
    memory: '6G',
    createdUnix: daysAgo(60),
    ready: true,
    running: false,
    cpuPct: 0,
    memUsedMb: 0,
    diskBytes: 1_400_000_000,
    port: 25566,
    players: 0,
    maxPlayers: 40,
    motd: 'Modded chaos',
    backupInterval: '',
    backupRetention: 5,
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
    gameVersion: '1.21.1',
    javaMajor: 21,
    memory: '4G',
    createdUnix: daysAgo(14),
    ready: false,
    running: false,
    cpuPct: 0,
    memUsedMb: 0,
    diskBytes: 210_000_000,
    players: 0,
    maxPlayers: 10,
    motd: 'One life.',
    backupInterval: '12h',
    backupRetention: 3,
    content: [],
    backups: [],
  },
];

/** The world resumed by the library's "continue playing". */
export const featured = instances[0];
