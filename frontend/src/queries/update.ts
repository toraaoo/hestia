/**
 * Self-update — the shell's own updater, not a daemon channel.
 *
 * The check is deliberately **not** automatic on mount: it reaches the network,
 * and an update is not something the app should nag about before the user asks.
 * The Settings surface triggers it.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';

import type { DesktopUpdate } from '../api/update';
import * as api from '../api/update';
import { mutation } from './core';
import { keys } from './keys';

export const updateQueries = {
  check: () =>
    queryOptions({
      queryKey: keys.update.check(),
      queryFn: () => api.check(),
      // A release feed does not change while a settings pane is open, and the
      // check costs a network round trip.
      staleTime: 5 * 60 * 1000,
      retry: false,
    }),
};

export const changelogQuery = () =>
  queryOptions({
    queryKey: keys.update.changelog(),
    queryFn: () => api.changelog(),
    // Compiled into the binary — it cannot change while the app is running.
    staleTime: Number.POSITIVE_INFINITY,
  });

export const updateMutations = {
  install: () =>
    mutation<void, void>({
      mutationKey: [...keys.update.all, 'install'],
      mutationFn: () => api.install(),
      invalidates: () => [keys.update.all],
    }),
};

/** `enabled: false` until the user asks — see the note above. */
export function useUpdateCheck(enabled: boolean) {
  return useQuery({ ...updateQueries.check(), enabled });
}

export function useInstallUpdate() {
  return useMutation(updateMutations.install());
}

export type { DesktopUpdate };
