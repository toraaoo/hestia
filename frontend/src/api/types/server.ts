/** Mirrors `crates/proto/src/server.rs`. */
import type { ConfigEntry } from './minecraft';
import type { ProcessInfo } from './process';

/**
 * A managed server: the stored record plus, when it has been started, the
 * supervised process snapshot.
 */
export interface ServerInfo {
  id: string;
  name: string;
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  javaMajor: number;
  createdUnix: number;
  /** False while the create job is still provisioning files. */
  ready: boolean;
  /** Allocated at create and stable thereafter — players connect to it. */
  gamePort?: number;
  /** True once RCON is configured. */
  console: boolean;
  process?: ProcessInfo;
  /**
   * Client-derived from the desktop icon store; absent when the entry keeps
   * its kind glyph. Not part of the wire record — the query hooks attach it.
   */
  iconUrl?: string;
}

/**
 * A server's static, informational view: descriptor, on-disk locations, and
 * footprint — everything independent of the live process. Fetched on demand
 * (the disk figure is a directory walk).
 */
export interface ServerDetails {
  id: string;
  name: string;
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  javaMajor: number;
  createdUnix: number;
  gamePort?: number;
  /** The entry root (`servers/<id>/`) — hestia's namespace. */
  entryDir: string;
  /** The game's working directory (`servers/<id>/data/`). */
  dataDir: string;
  /** The entry's total on-disk footprint, in bytes. */
  diskBytes: number;
}

/** Server List Ping snapshot; only a running server answers. */
export interface ServerPingResult {
  playersOnline: number;
  playersMax: number;
  motd: string;
  version: string;
}

export interface ServerCreateParams {
  /** Display name; defaults to `<flavor>-<version>` when empty. */
  name?: string;
  flavor: string;
  version: string;
  loaderVersion?: string;
  /** The caller confirms the user accepted the Minecraft EULA. */
  eula: boolean;
  /** Pin the game port; omitted picks the lowest free one. */
  port?: number;
  /** Create-time settings (memory, jvm-args, any `server.properties` key). */
  config?: ConfigEntry[];
}

export interface ServerUpdateParams {
  /** Server name or id. */
  server: string;
  version: string;
  loaderVersion?: string;
  /** The caller confirms the risk of moving to an older version. */
  allowDowngrade?: boolean;
}
