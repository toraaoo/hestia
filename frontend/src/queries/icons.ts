/** Custom entry icons — the `icons_*` shell commands as queries/mutations. */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import * as api from '../api/icons';
import { mutation } from './core';
import { keys } from './keys';

export const iconQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.icons.list(),
      queryFn: () => api.list(),
    }),
};

export const iconMutations = {
  set: () =>
    mutation<api.IconEntry, { entryId: string; sourcePath: string }>({
      mutationKey: [...keys.icons.all, 'set'],
      mutationFn: ({ entryId, sourcePath }) => api.set(entryId, sourcePath),
      invalidates: () => [keys.icons.all],
    }),
  remove: () =>
    mutation<void, string>({
      mutationKey: [...keys.icons.all, 'remove'],
      mutationFn: (entryId) => api.remove(entryId),
      invalidates: () => [keys.icons.all],
    }),
};

export function useEntryIcons() {
  return useQuery(iconQueries.list());
}

/** One entry's icon URL, or undefined when it keeps the default glyph. */
export function useEntryIconUrl(entryId: string): string | undefined {
  const icons = useEntryIcons();
  const entry = icons.data?.[entryId];
  return entry ? api.iconUrl(entry) : undefined;
}

export function useSetEntryIcon() {
  return useMutation(iconMutations.set());
}

export function useRemoveEntryIcon() {
  return useMutation(iconMutations.remove());
}
