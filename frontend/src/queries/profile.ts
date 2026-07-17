/**
 * `profile.*` — global content profiles: queries/mutations plus their 1:1
 * hooks. Applying a profile into an instance lives with the instance
 * mutations (`useApplyInstanceProfile`), where its invalidations belong.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
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

export function useGlobalProfiles() {
  return useQuery(profileQueries.list());
}

export function useCreateGlobalProfile() {
  return useMutation(profileMutations.create());
}

export function useRemoveGlobalProfile() {
  return useMutation(profileMutations.remove());
}

export function useEditGlobalProfile() {
  return useMutation(profileMutations.edit());
}
