/** The `sync.*` channels — the shared settings/config target set (instance-only). */
import { call } from './core/ipc';
import type { SyncConfig, SyncTargets } from './types/sync';

export function get(): Promise<SyncConfig> {
  return call('sync.get');
}

/** Replace the target set wholesale; the daemon validates each path. */
export function set(targets: SyncTargets): Promise<SyncConfig> {
  return call('sync.set', { targets });
}
