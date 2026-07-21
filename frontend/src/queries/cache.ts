/** `cache.*` — query/mutation factories, consumed through useQuery/useMutation. */
import { queryOptions } from '@tanstack/react-query';
import type { CacheUsage } from '../api';
import * as api from '../api/cache';
import { mutation } from './core';
import { keys } from './keys';

export const cacheQueries = {
  info: () =>
    queryOptions({
      queryKey: keys.cache.info(),
      queryFn: () => api.info(),
    }),
  list: () =>
    queryOptions({
      queryKey: keys.cache.list(),
      queryFn: () => api.list(),
    }),
};

export const cacheMutations = {
  /** Resolves with what was reclaimed. */
  clear: () =>
    mutation<CacheUsage>({
      mutationKey: [...keys.cache.all, 'clear'],
      mutationFn: () => api.clear(),
      invalidates: () => [keys.cache.all],
    }),
};
