/** The `sync.*` channels — per-kind shared settings/config target sets. */
import { call } from './core/ipc';
import type { SyncConfig, SyncKind, SyncTargets } from './types/sync';

export function get(): Promise<SyncConfig> {
  return call('sync.get');
}

/** Replace one kind's target set wholesale; the daemon validates each path. */
export function set(kind: SyncKind, targets: SyncTargets): Promise<SyncConfig> {
  return call('sync.set', { kind, targets });
}
