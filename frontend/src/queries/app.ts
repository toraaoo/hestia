/** `app.*` / `health.*` — queries plus their 1:1 hooks. */
import { queryOptions, useQuery } from '@tanstack/react-query';
import * as api from '../api/app';
import { keys } from './keys';

export const appQueries = {
  info: () =>
    queryOptions({
      queryKey: keys.app.info(),
      queryFn: () => api.info(),
      staleTime: Number.POSITIVE_INFINITY,
    }),
  ping: () =>
    queryOptions({
      queryKey: keys.app.ping(),
      queryFn: () => api.ping(),
      staleTime: 0,
    }),
};

export function useAppInfo() {
  return useQuery(appQueries.info());
}

export function usePing() {
  return useQuery(appQueries.ping());
}
