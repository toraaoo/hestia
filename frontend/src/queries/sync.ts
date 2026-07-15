/** `sync.*` — the per-kind shared settings/config target sets. */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { SyncConfig, SyncKind, SyncTargets } from '../api';
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
  /** Replace one kind's target set wholesale; the daemon validates paths. */
  set: () =>
    mutation<SyncConfig, { kind: SyncKind; targets: SyncTargets }>({
      mutationKey: [...keys.sync.all, 'set'],
      mutationFn: ({ kind, targets }) => api.set(kind, targets),
      invalidates: () => [keys.sync.all],
    }),
};

export function useSyncConfig() {
  return useQuery(syncQueries.config());
}

export function useSetSyncTargets() {
  return useMutation(syncMutations.set());
}
