/** Mirrors `crates/proto/src/sync.rs`. */

/** Game-relative paths shared across instances: files copied, folders linked. */
export interface SyncTargets {
  files: string[];
  folders: string[];
}

export interface SyncConfig {
  shared_dir: string;
  targets: SyncTargets;
}

/**
 * One folder target's link state on one instance. `pending` links at the next
 * launch; `cannot_link` needs `instance.sync.adopt`.
 */
export type LinkState = 'linked' | 'pending' | 'cannot_link';

export interface TargetLinkState {
  target: string;
  state: LinkState;
}

export interface InstanceSyncStatus {
  id: string;
  name: string;
  targets: TargetLinkState[];
}
