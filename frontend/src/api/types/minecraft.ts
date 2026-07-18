/** Mirrors `crates/proto/src/minecraft.rs`. */
import type { Checksum } from './download';

export interface Flavor {
  id: string;
  name: string;
}

/** One key/value setting, shared by the server/instance config channels. */
export interface ConfigEntry {
  key: string;
  value: string;
}

export type VersionKind = 'release' | 'snapshot' | 'old_beta' | 'old_alpha';

export interface GameVersion {
  id: string;
  kind: VersionKind;
  stable: boolean;
}

export interface Artifact {
  url: string;
  filename: string;
  size: number;
  checksum?: Checksum;
}

export interface Library {
  name: string;
  path: string;
  artifact: Artifact;
}

export interface AssetIndex {
  id: string;
  artifact: Artifact;
  totalSize: number;
}

export interface ServerProfile {
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  primary: Artifact;
  libraries: Library[];
  javaMajor: number;
  mainClass: string;
}

export interface InstanceProfile {
  flavor: string;
  gameVersion: string;
  loaderVersion?: string;
  client: Artifact;
  libraries: Library[];
  assetIndex: AssetIndex;
  javaMajor: number;
  mainClass: string;
  jvmArgs: string[];
  gameArgs: string[];
}

export interface ResolveParams {
  flavor: string;
  version: string;
  loaderVersion?: string;
}

export type ProvisionPhase =
  | 'resolving'
  | 'backup'
  | 'java'
  | 'server'
  | 'client'
  | 'libraries'
  | 'assets'
  | 'content';

/**
 * Progress for a provisioning job. `current`/`total` are bytes for a
 * single-artifact phase and completed/total counts for libraries/assets; a
 * multi-unit phase (a content batch) also carries `item` of `items`.
 */
export interface ProvisionProgress {
  phase: ProvisionPhase;
  current: number;
  total: number;
  detail?: string;
  item?: number;
  items?: number;
}

/**
 * Whether moving `from` → `to` is a downgrade, judged by position in the
 * flavor's newest-first catalogue (mirrors `proto::minecraft::downgrade_between`).
 * `null` when either version is not listed.
 */
export function downgradeBetween(
  versions: GameVersion[],
  from: string,
  to: string,
): boolean | null {
  const fromIndex = versions.findIndex((v) => v.id === from);
  const toIndex = versions.findIndex((v) => v.id === to);
  if (fromIndex < 0 || toIndex < 0) return null;
  return toIndex > fromIndex;
}
