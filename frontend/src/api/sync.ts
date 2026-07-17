/**
 * The `sync.*` channels — the shared settings/config target set
 * (instance-only): files copied, folders linked into the shared store.
 */
import { call } from './core/ipc';
import type { InstanceSyncStatus, SyncConfig, SyncTargets } from './types/sync';

export function get(): Promise<SyncConfig> {
  return call('sync.get');
}

/** Replace the target set wholesale; the daemon validates each path. */
export function set(targets: SyncTargets): Promise<SyncConfig> {
  return call('sync.set', { targets });
}

/** Every instance's per-folder-target link state. */
export async function status(): Promise<InstanceSyncStatus[]> {
  const result = await call<{ instances: InstanceSyncStatus[] }>('sync.status');
  return result.instances;
}

/**
 * Adopt a stopped instance's folder contents into the shared store (every
 * folder target when `targets` is empty). Returns the targets linked after
 * the call; a store collision refuses that target with the names.
 */
export async function adopt(
  instance: string,
  targets: string[] = [],
): Promise<string[]> {
  const result = await call<{ adopted: string[] }>('instance.sync.adopt', {
    instance,
    targets,
  });
  return result.adopted;
}
