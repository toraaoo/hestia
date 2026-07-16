/**
 * Static stand-in data for the launcher layout, shaped after the daemon's real
 * surface (see `api/*` and `crates/proto`) so the pages mirror actual
 * functionality: instances/servers with content, worlds, backups and config;
 * Modrinth content browse; Java runtimes; accounts. Nothing talks to a backend.
 */

export type ContentKind =
  | 'mod'
  | 'resourcepack'
  | 'shader'
  | 'datapack'
  | 'modpack';

export interface Account {
  name: string;
  uuid: string;
  active: boolean;
}

export const accounts: Account[] = [
  {
    name: 'toraaoo',
    uuid: '8f0e1c2a-3b4d-5e6f-7a8b-9c0d1e2f3a4b',
    active: true,
  },
  {
    name: 'hearthguest',
    uuid: '11112222-3333-4444-5555-666677778888',
    active: false,
  },
];

export const account = accounts.find((a) => a.active) ?? accounts[0];

export interface DaemonStatus {
  connected: boolean;
  version: string;
  socket: string;
  uptime: string;
}

export const daemon: DaemonStatus = {
  connected: true,
  version: '0.0.1',
  socket: 'hestiad.sock',
  uptime: '3h 12m',
};

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
  content: InstalledContent[];
  worlds: string[];
  backups: Backup[];
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
    backups: [
      {
        id: 'b1',
        created_unix: hoursAgo(6),
        kind: 'manual',
        size_bytes: 184_320_000,
      },
      {
        id: 'b2',
        created_unix: daysAgo(3),
        kind: 'update',
        size_bytes: 172_800_000,
      },
    ],
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
    content: [],
    worlds: ['Survival'],
    backups: [],
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
    backups: [
      {
        id: 'b3',
        created_unix: daysAgo(20),
        kind: 'manual',
        size_bytes: 512_000_000,
      },
    ],
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
    content: [],
    worlds: [],
    backups: [],
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

/** The world resumed by the library's "continue playing". */
export const featured = instances[0];

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
  updated_unix: number;
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
    updated_unix: daysAgo(5),
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
    updated_unix: daysAgo(9),
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
    updated_unix: daysAgo(2),
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
    updated_unix: daysAgo(15),
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
    updated_unix: daysAgo(7),
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
    updated_unix: daysAgo(30),
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
    updated_unix: daysAgo(22),
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
    updated_unix: daysAgo(12),
  },
];

export const getProject = (id: string) =>
  contentProjects.find((p) => p.id === id);

export type SkinVariant = 'classic' | 'slim';

/** A cape the account owns, from the Mojang profile. */
export interface Cape {
  id: string;
  name: string;
  texture: string;
}

export const capes: Cape[] = [
  { id: 'vanilla', name: 'Vanilla', texture: '/capes/vanilla.png' },
  { id: 'home', name: 'Home', texture: '/capes/home.png' },
  {
    id: 'cherry-blossom',
    name: 'Cherry Blossom',
    texture: '/capes/cherry-blossom.png',
  },
];

export const getCape = (id?: string) => capes.find((c) => c.id === id);

/** A skin in the library: bundled defaults plus the user's saved skins. */
export interface Skin {
  id: string;
  name: string;
  variant: SkinVariant;
  /** URL or data URL of the 64x64 skin texture. */
  texture: string;
  cape_id?: string;
  source: 'default' | 'custom';
}

/** Each default character with its canonical model, as the vanilla launcher presents them. */
const defaultSkinModels: [string, SkinVariant][] = [
  ['Steve', 'classic'],
  ['Alex', 'slim'],
  ['Ari', 'classic'],
  ['Efe', 'slim'],
  ['Kai', 'classic'],
  ['Makena', 'slim'],
  ['Noor', 'slim'],
  ['Sunny', 'classic'],
  ['Zuri', 'classic'],
];

export const defaultSkins: Skin[] = defaultSkinModels.map(([name, variant]) => {
  const slug = name.toLowerCase();
  return {
    id: `default-${slug}`,
    name,
    variant,
    texture: `/skins/${slug}${variant === 'slim' ? '-slim' : ''}.png`,
    source: 'default',
  };
});

export const customSkins: Skin[] = [
  {
    id: 'custom-adventurer',
    name: 'Adventurer',
    variant: 'classic',
    texture: '/skins/zuri.png',
    cape_id: 'vanilla',
    source: 'custom',
  },
  {
    id: 'custom-netherborn',
    name: 'Netherborn',
    variant: 'slim',
    texture: '/skins/efe-slim.png',
    cape_id: 'home',
    source: 'custom',
  },
];

/** The skin currently applied to the signed-in account. */
export const equippedSkinId = 'custom-adventurer';

/** An installed Java runtime, from `java.list`. */
export interface JavaRuntime {
  vendor: string;
  major: number;
  version: string;
  in_use: boolean;
}

export const javaRuntimes: JavaRuntime[] = [
  { vendor: 'Temurin', major: 21, version: '21.0.5+11', in_use: true },
  { vendor: 'Temurin', major: 17, version: '17.0.13+11', in_use: true },
  { vendor: 'Temurin', major: 8, version: '8u432-b06', in_use: false },
];

/** An installable Java major, from `java.releases`. */
export interface JavaRelease {
  major: number;
  lts: boolean;
  installed: boolean;
}

export const javaReleases: JavaRelease[] = [
  { major: 8, lts: true, installed: true },
  { major: 11, lts: true, installed: false },
  { major: 17, lts: true, installed: true },
  { major: 21, lts: true, installed: true },
  { major: 23, lts: false, installed: false },
];
