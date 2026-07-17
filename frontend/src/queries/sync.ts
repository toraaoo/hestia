/** `sync.*` — the shared settings/config target set (instance-only). */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { SyncConfig, SyncTargets } from '../api';
import * as api from '../api/sync';
import { mutation } from './core';
import { keys } from './keys';

export const syncQueries = {
  config: () =>
    queryOptions({
      queryKey: keys.sync.config(),
      queryFn: () => api.get(),
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
};

export function useSyncConfig() {
  return useQuery(syncQueries.config());
}

export function useSetSyncTargets() {
  return useMutation(syncMutations.set());
}
