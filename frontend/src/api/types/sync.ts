/** Mirrors `crates/proto/src/sync.rs`. */

/** Game-relative paths shared across instances. */
export interface SyncTargets {
  files: string[];
  folders: string[];
}

export interface SyncConfig {
  shared_dir: string;
  targets: SyncTargets;
}
