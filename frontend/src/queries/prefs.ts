import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import * as api from '../api/prefs';
import { mutation } from './core';
import { keys } from './keys';

export const prefsQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.prefs.list(),
      queryFn: () => api.list(),
    }),
};

export const prefsMutations = {
  set: () =>
    mutation<void, { key: string; value: unknown }>({
      mutationKey: [...keys.prefs.all, 'set'],
      mutationFn: ({ key, value }) => api.set(key, value),
      invalidates: () => [keys.prefs.all],
    }),
  remove: () =>
    mutation<void, string>({
      mutationKey: [...keys.prefs.all, 'remove'],
      mutationFn: (key) => api.remove(key),
      invalidates: () => [keys.prefs.all],
    }),
};

export function usePrefs() {
  const query = useQuery(prefsQueries.list());
  const setMutation = useMutation(prefsMutations.set());
  const removeMutation = useMutation(prefsMutations.remove());

  const prefs = query.data ?? {};

  return {
    prefs,
    ready: !query.isPending,
    get: <T>(key: string, fallback: T): T =>
      key in prefs ? (prefs[key] as T) : fallback,
    set: (key: string, value: unknown) => setMutation.mutate({ key, value }),
    remove: (key: string) => removeMutation.mutate(key),
  };
}
