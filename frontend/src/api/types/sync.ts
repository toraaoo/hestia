/** Mirrors `crates/proto/src/sync.rs`. */

export type SyncKind = 'server' | 'instance';

/** Game-relative paths shared across entries of one kind. */
export interface SyncTargets {
  files: string[];
  folders: string[];
}

export interface SyncConfig {
  shared_dir: string;
  servers: SyncTargets;
  instances: SyncTargets;
}
