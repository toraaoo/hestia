/** Mirrors `crates/proto/src/instance.rs`. */
import type { ConfigEntry } from './minecraft';
import type { ProcessInfo } from './process';

/**
 * A managed instance: the stored record plus, when launched, its live
 * sessions — an instance can run more than once concurrently.
 */
export interface InstanceInfo {
  id: string;
  name: string;
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  javaMajor: number;
  createdUnix: number;
  sessions?: ProcessInfo[];
  /**
   * Client-derived from the desktop icon store; absent when the entry keeps
   * its kind glyph. Not part of the wire record — the query hooks attach it.
   */
  iconUrl?: string;
}

/**
 * An instance's static, informational view: descriptor, on-disk locations, and
 * footprint — everything independent of the live sessions. Fetched on demand
 * (the disk figure is a directory walk).
 */
export interface InstanceDetails {
  id: string;
  name: string;
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  javaMajor: number;
  createdUnix: number;
  /** The entry root (`instances/<id>/`) — hestia's namespace. */
  entryDir: string;
  /** The game's working directory (`instances/<id>/data/`). */
  dataDir: string;
  /** The entry's total on-disk footprint, in bytes. */
  diskBytes: number;
}

export interface InstanceCreateParams {
  /** Display name; defaults to `<flavor>-<version>` when empty. */
  name?: string;
  flavor: string;
  version: string;
  loaderVersion?: string;
  /** Create-time settings (memory, jvm-args). */
  config?: ConfigEntry[];
}

export interface InstanceUpdateParams {
  /** Instance name or id. */
  instance: string;
  version: string;
  loaderVersion?: string;
  /** The caller confirms the risk of moving to an older version. */
  allowDowngrade?: boolean;
}

export interface InstanceLaunchParams {
  instance: string;
  /** Account name or uuid; empty picks the sole signed-in account. */
  account?: string;
  /** Launch another session even when one is already running. */
  newSession?: boolean;
  /**
   * A profile override for this launch only: empty uses the active profile,
   * the literal `none` launches with no profile.
   */
  profile?: string;
}

export interface InstanceLaunchDone {
  id: string;
  processId: string;
  pid: number;
}

/**
 * A named selection over the instance's installed content pool (mods,
 * resourcepacks, shaders — never datapacks). Members are pool filenames.
 * No profile active = every pool item is mirrored.
 */
export interface ContentProfile {
  name: string;
  members: string[];
  /**
   * Whether the profile owns a captured settings store: launches under it
   * sync settings against the profile's own store instead of the global one.
   */
  captured: boolean;
}

export interface InstanceProfiles {
  /** The active profile's name; empty when none is active. */
  active: string;
  profiles: ContentProfile[];
}
