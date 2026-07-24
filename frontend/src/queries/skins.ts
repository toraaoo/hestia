/**
 * `skin.*` / `cape.*` — query/mutation factories, consumed through
 * useQuery/useMutation. One query
 * serves the whole picker (skins and capes come from a single profile fetch);
 * every change invalidates it, since equipping, uploading, or resetting all
 * reshape which entry is equipped. `account` is a name or uuid; empty (the
 * default) targets the default account.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type { Skin, SkinList, SkinVariant } from '../api';
import * as api from '../api/skins';
import { queryClient } from './client';
import { mutation } from './core';
import { keys } from './keys';

export const skinQueries = {
  list: (account = '') =>
    queryOptions({
      queryKey: keys.skins.list(account),
      queryFn: () => api.list(account),
    }),
};

// Exact transforms skip the settle refetch (a Mojang round trip); equip keeps
// it, since the daemon may mint a preserved library row. Keyed by the target
// account so a non-default picker never writes the wrong cache entry.
function optimisticList(
  account: string,
  update: (list: SkinList) => SkinList,
): (() => void) | undefined {
  const key = keys.skins.list(account);
  void queryClient.cancelQueries({ queryKey: key });
  const previous = queryClient.getQueryData<SkinList>(key);
  if (!previous) return undefined;
  queryClient.setQueryData<SkinList>(key, update(previous));
  return () => queryClient.setQueryData(key, previous);
}

// An external row exists only while equipped; equipping another drops it.
function equipSkinInList(list: SkinList, key: string): SkinList {
  return {
    ...list,
    skins: list.skins
      .filter((s) => s.source !== 'external' || s.key === key)
      .map((s) => ({ ...s, equipped: s.key === key })),
  };
}

/**
 * The equipped skin's texture hash for `account` (empty = default), read from
 * the picker cache without fetching — so it's known only once the skin surface
 * has loaded, and changes the moment an equip/reset settles there. Callers use
 * it to cache-bust the mc-heads avatar, whose url is otherwise uuid-only and so
 * pinned in the browser image cache across a skin change.
 */
export function useEquippedSkinKey(account = ''): string | undefined {
  const { data } = useQuery({
    ...skinQueries.list(account),
    enabled: false,
    notifyOnChangeProps: ['data'],
  });
  return data?.skins.find((s) => s.equipped)?.key;
}

export const skinMutations = {
  add: () =>
    mutation<
      Skin,
      { account?: string; name?: string; variant: SkinVariant; data: string }
    >({
      mutationKey: [...keys.skins.all, 'add'],
      mutationFn: (params) => api.add(params),
      onSuccess: (skin, { account }) =>
        queryClient.setQueryData<SkinList>(
          keys.skins.list(account ?? ''),
          (prev) =>
            prev
              ? {
                  ...prev,
                  skins: [
                    skin,
                    ...prev.skins
                      .filter((s) => s.key !== skin.key)
                      .map((s) => ({ ...s, equipped: false })),
                  ],
                }
              : prev,
        ),
      invalidates: () => [keys.skins.all],
    }),
  update: () =>
    mutation<
      Skin,
      { account?: string; key: string; name: string; variant: SkinVariant }
    >({
      mutationKey: [...keys.skins.all, 'update'],
      mutationFn: (params) => api.update(params),
      optimistic: ({ key, name, variant, account }) =>
        optimisticList(account ?? '', (list) => ({
          ...list,
          skins: list.skins.map((s) =>
            s.key === key ? { ...s, name, variant } : s,
          ),
        })),
    }),
  equip: () =>
    mutation<void, { key: string; account?: string }>({
      mutationKey: [...keys.skins.all, 'equip'],
      mutationFn: ({ key, account }) => api.equip(key, account),
      optimistic: ({ key, account }) =>
        optimisticList(account ?? '', (list) => equipSkinInList(list, key)),
      invalidates: () => [keys.skins.all],
    }),
  reset: () =>
    mutation<void, { account?: string } | undefined>({
      mutationKey: [...keys.skins.all, 'reset'],
      mutationFn: (params) => api.reset(params?.account),
      invalidates: () => [keys.skins.all],
    }),
  remove: () =>
    mutation<void, { key: string; account?: string }>({
      mutationKey: [...keys.skins.all, 'remove'],
      mutationFn: ({ key }) => api.remove(key),
      optimistic: ({ key, account }) =>
        optimisticList(account ?? '', (list) => ({
          ...list,
          skins: list.skins.filter((s) => s.key !== key),
        })),
    }),
  equipCape: () =>
    mutation<void, { cape: string; account?: string }>({
      mutationKey: [...keys.skins.all, 'cape', 'equip'],
      mutationFn: ({ cape, account }) => api.equipCape(cape, account),
      optimistic: ({ cape, account }) =>
        optimisticList(account ?? '', (list) => ({
          ...list,
          capes: list.capes.map((c) => ({ ...c, equipped: c.id === cape })),
        })),
    }),
  clearCape: () =>
    mutation<void, { account?: string } | undefined>({
      mutationKey: [...keys.skins.all, 'cape', 'clear'],
      mutationFn: (params) => api.clearCape(params?.account),
      optimistic: (params) =>
        optimisticList(params?.account ?? '', (list) => ({
          ...list,
          capes: list.capes.map((c) => ({ ...c, equipped: false })),
        })),
    }),
};
