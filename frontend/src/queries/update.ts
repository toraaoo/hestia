/**
 * `update.*` — the check is deliberately **not** automatic on mount: it reaches
 * the network, and an update is not something the app should nag about before
 * the user asks. The Settings surface triggers it.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';

import type { DownloadProgress } from '../api/types/download';
import type {
  UpdateApplyResult,
  UpdateCheckResult,
  UpdateDoneEvent,
} from '../api/types/update';
import * as api from '../api/update';
import { mutation } from './core';
import { jobMutation, useJobMutation } from './jobs';
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
  download: () =>
    jobMutation<UpdateDoneEvent, void, DownloadProgress>({
      mutationKey: [...keys.update.all, 'download'],
      meta: () => ({ kind: 'update.download', label: 'download update' }),
      run: (_variables, job) => api.download(job),
    }),
  apply: () =>
    mutation<UpdateApplyResult, string>({
      mutationKey: [...keys.update.all, 'apply'],
      mutationFn: (path) => api.apply(path),
      invalidates: () => [keys.update.all],
    }),
};

/** `enabled: false` until the user asks — see the note above. */
export function useUpdateCheck(enabled: boolean) {
  return useQuery({ ...updateQueries.check(), enabled });
}

export function useDownloadUpdate() {
  return useJobMutation(updateMutations.download());
}

export function useApplyUpdate() {
  return useMutation(updateMutations.apply());
}

export type { UpdateCheckResult };
