/**
 * `sync.*` — the shared settings/config target set (instance-only): files
 * copied, folders linked into the shared store.
 */
import { queryOptions } from '@tanstack/react-query';
import type { SyncConfig, SyncShareResult, SyncTargets } from '../api';
import * as api from '../api/sync';
import { mutation } from './core';
import { keys } from './keys';

export const syncQueries = {
  config: () =>
    queryOptions({
      queryKey: keys.sync.config(),
      queryFn: () => api.get(),
    }),
  /** Every instance's per-folder-target link state. */
  status: () =>
    queryOptions({
      queryKey: keys.sync.status(),
      queryFn: () => api.status(),
    }),
};

export const syncMutations = {
  /** Replace the target set wholesale; the daemon validates paths. */
  set: () =>
    mutation<SyncConfig, SyncTargets>({
      mutationKey: [...keys.sync.all, 'set'],
      mutationFn: (targets) => api.set(targets),
      invalidates: () => [keys.sync.all],
    }),
  /**
   * Whether one instance takes part at all. Files move either way, so the
   * instance must be stopped and the result carries what it cost.
   */
  share: (id: string) =>
    mutation<SyncShareResult, boolean>({
      mutationKey: [...keys.instances.detail(id), 'sync', 'share'],
      mutationFn: (enabled) => api.share(id, enabled),
      invalidates: () => [keys.sync.all, keys.instances.detail(id)],
    }),
  /** Adopt a stopped instance's folders into the store; empty = all. */
  adopt: (id: string) =>
    mutation<string[], string[] | undefined>({
      mutationKey: [...keys.instances.detail(id), 'sync', 'adopt'],
      mutationFn: (targets) => api.adopt(id, targets),
      invalidates: () => [keys.sync.all, keys.instances.detail(id)],
    }),
};
