/**
 * `process.*` — queries/mutations plus their 1:1 hooks, thin over the
 * daemon's supervisor. Servers and instances have their own richer hooks;
 * these are the raw per-process surface.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { ProcessSpec } from '../api';
import * as api from '../api/process';
import { mutation } from './core';
import { keys } from './keys';
import { type LogsOptions, type LogsResult, useFollowedLogs } from './logs';

export const processQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.processes.list(),
      queryFn: () => api.list(),
    }),
  status: (id: string) =>
    queryOptions({
      queryKey: keys.processes.status(id),
      queryFn: () => api.status(id),
    }),
  logs: (id: string, tail?: number) =>
    queryOptions({
      queryKey: keys.processes.logs(id, tail),
      queryFn: () => api.logs(id, tail),
    }),
};

export const processMutations = {
  start: () =>
    mutation<{ id: string; pid: number }, ProcessSpec>({
      mutationKey: [...keys.processes.all, 'start'],
      mutationFn: (spec) => api.start(spec),
      invalidates: () => [keys.processes.all],
    }),
  stop: () =>
    mutation<void, string>({
      mutationKey: [...keys.processes.all, 'stop'],
      mutationFn: (id) => api.stop(id),
      invalidates: () => [keys.processes.all],
    }),
};

export function useProcesses() {
  return useQuery(processQueries.list());
}

export function useProcess(id: string) {
  return useQuery(processQueries.status(id));
}

export function useProcessLogs(
  id: string,
  options: LogsOptions = {},
): LogsResult {
  const query = useQuery({
    ...processQueries.logs(id, options.tail),
    staleTime: options.follow ? Number.POSITIVE_INFINITY : undefined,
  });
  return useFollowedLogs(
    query,
    options.follow ? (processId) => processId === id : null,
    options.limit,
  );
}

export function useStartProcess() {
  return useMutation(processMutations.start());
}

export function useStopProcess() {
  return useMutation(processMutations.stop());
}
