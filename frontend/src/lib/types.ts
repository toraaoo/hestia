/**
 * Frontend view models, shaped after the daemon's proto vocabulary
 * (proto::server / proto::instances / proto::content) so wiring the
 * real client later is a data-source swap, not a refactor.
 */

export type TileName =
  | "tile-grass"
  | "tile-forge"
  | "tile-diamond"
  | "tile-nether"
  | "tile-ocean"
  | "tile-end"
  | "tile-server"
  | "tile-sky";

export type Loader = "Vanilla" | "Fabric" | "Forge" | "Quilt" | "NeoForge" | "Paper";

export interface Instance {
  id: string;
  name: string;
  tile: TileName;
  loader: Loader;
  version: string;
  lastPlayed: string;
  playtime: string;
  running: boolean;
  pinned: boolean;
  modCount: number;
  worldCount: number;
  sizeOnDisk: string;
  memoryGb: number;
  description: string;
}

export interface Server {
  id: string;
  name: string;
  tile: TileName;
  port: number;
  version: string;
  players: number;
  maxPlayers: number;
  ramGb: number;
  ramMaxGb: number;
  tps: number;
  uptime: string;
}

export interface InstalledMod {
  name: string;
  tile: TileName;
  summary: string;
  enabled: boolean;
}

export interface WorldSave {
  name: string;
  tile: TileName;
  summary: string;
}

export type LogLevel = "INFO" | "WARN" | "ERROR";
export type LogLine = readonly [time: string, level: LogLevel, message: string];

export type ContentSource = "modrinth" | "curseforge";

export interface ContentProject {
  name: string;
  author: string;
  tile: TileName;
  description: string;
  downloads: number;
  likes?: number;
  source: ContentSource;
  loaders: Loader[];
  installed?: boolean;
}

export type LibraryView = "grid" | "list";
