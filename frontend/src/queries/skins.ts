/**
 * `skin.*` / `cape.*` — queries/mutations plus their 1:1 hooks. One query
 * serves the whole picker (skins and capes come from a single profile fetch);
 * every change invalidates it, since equipping, uploading, or resetting all
 * reshape which entry is equipped. `account` is a name or uuid; empty (the
 * default) targets the default account.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { Skin, SkinVariant } from '../api';
import * as api from '../api/skins';
import { mutation } from './core';
import { keys } from './keys';

export const skinQueries = {
  list: (account = '') =>
    queryOptions({
      queryKey: keys.skins.list(account),
      queryFn: () => api.list(account),
    }),
};

export const skinMutations = {
  add: () =>
    mutation<
      Skin,
      { account?: string; name?: string; variant: SkinVariant; data: string }
    >({
      mutationKey: [...keys.skins.all, 'add'],
      mutationFn: (params) => api.add(params),
      invalidates: () => [keys.skins.all],
    }),
  update: () =>
    mutation<
      Skin,
      { account?: string; key: string; name: string; variant: SkinVariant }
    >({
      mutationKey: [...keys.skins.all, 'update'],
      mutationFn: (params) => api.update(params),
      invalidates: () => [keys.skins.all],
    }),
  equip: () =>
    mutation<void, { key: string; account?: string }>({
      mutationKey: [...keys.skins.all, 'equip'],
      mutationFn: ({ key, account }) => api.equip(key, account),
      invalidates: () => [keys.skins.all],
    }),
  reset: () =>
    mutation<void, { account?: string } | undefined>({
      mutationKey: [...keys.skins.all, 'reset'],
      mutationFn: (params) => api.reset(params?.account),
      invalidates: () => [keys.skins.all],
    }),
  remove: () =>
    mutation<void, string>({
      mutationKey: [...keys.skins.all, 'remove'],
      mutationFn: (key) => api.remove(key),
      invalidates: () => [keys.skins.all],
    }),
  equipCape: () =>
    mutation<void, { cape: string; account?: string }>({
      mutationKey: [...keys.skins.all, 'cape', 'equip'],
      mutationFn: ({ cape, account }) => api.equipCape(cape, account),
      invalidates: () => [keys.skins.all],
    }),
  clearCape: () =>
    mutation<void, { account?: string } | undefined>({
      mutationKey: [...keys.skins.all, 'cape', 'clear'],
      mutationFn: (params) => api.clearCape(params?.account),
      invalidates: () => [keys.skins.all],
    }),
};

export function useSkins(account = '') {
  return useQuery(skinQueries.list(account));
}

export function useAddSkin() {
  return useMutation(skinMutations.add());
}

export function useUpdateSkin() {
  return useMutation(skinMutations.update());
}

export function useEquipSkin() {
  return useMutation(skinMutations.equip());
}

export function useResetSkin() {
  return useMutation(skinMutations.reset());
}

export function useRemoveSkin() {
  return useMutation(skinMutations.remove());
}

export function useEquipCape() {
  return useMutation(skinMutations.equipCape());
}

export function useClearCape() {
  return useMutation(skinMutations.clearCape());
}
