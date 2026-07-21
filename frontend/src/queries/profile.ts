/**
 * `profile.*` — global content profiles: query/mutation factories, consumed
 * through useQuery/useMutation. Applying a profile into an instance lives with
 * the instance mutations (`instanceMutations.profiles.apply`), where its
 * invalidations belong.
 */
import { queryOptions } from '@tanstack/react-query';
import type { GlobalProfile } from '../api';
import * as api from '../api/profile';
import { mutation } from './core';
import { keys } from './keys';

export const profileQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.profiles.list(),
      queryFn: () => api.list(),
    }),
};

export const profileMutations = {
  /** The name is slugged (`My QoL` becomes `my-qol`). */
  create: () =>
    mutation<GlobalProfile, string>({
      mutationKey: [...keys.profiles.all, 'create'],
      mutationFn: (name) => api.create(name),
      invalidates: () => [keys.profiles.all],
    }),
  remove: () =>
    mutation<void, string>({
      mutationKey: [...keys.profiles.all, 'remove'],
      mutationFn: (name) => api.remove(name),
      invalidates: () => [keys.profiles.all],
    }),
  /** Adds resolve through the content registry, so this can take a moment. */
  edit: () =>
    mutation<
      GlobalProfile,
      { name: string; source?: string; add?: string[]; remove?: string[] }
    >({
      mutationKey: [...keys.profiles.all, 'edit'],
      mutationFn: ({ name, ...options }) => api.edit(name, options),
      invalidates: () => [keys.profiles.all],
    }),
};
