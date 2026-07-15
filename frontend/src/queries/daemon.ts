/** `daemon.*` — queries/mutations plus their 1:1 hooks. */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import * as api from '../api/daemon';
import { mutation } from './core';
import { keys } from './keys';

export const daemonQueries = {
  status: () =>
    queryOptions({
      queryKey: keys.daemon.status(),
      queryFn: () => api.status(),
    }),
};

export const daemonMutations = {
  /** Without `stopProcesses`, supervised workloads keep running. */
  stop: () =>
    mutation<boolean, { stopProcesses: boolean }>({
      mutationKey: [...keys.daemon.all, 'stop'],
      mutationFn: ({ stopProcesses }) => api.stop(stopProcesses),
      invalidates: () => [keys.daemon.all],
    }),
};

export function useDaemonStatus() {
  return useQuery(daemonQueries.status());
}

export function useStopDaemon() {
  return useMutation(daemonMutations.stop());
}
