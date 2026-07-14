/** The small app-level domains: one hook each, query plus bound actions. */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import type { SyncKind, SyncTargets } from '../api';
import { app, cache, config, daemon, sync } from '../api';
import { sweeper } from './client';
import { keys } from './keys';

export function useAppInfo() {
  return useQuery({
    queryKey: keys.app,
    queryFn: app.info,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useDaemon() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.daemon, queryFn: daemon.status });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.daemon);
    return {
      /** Without `stopProcesses`, supervised workloads keep running. */
      stop: (stopProcesses = false) => done(daemon.stop(stopProcesses)),
    };
  }, [queryClient]);
  return { ...query, ...actions };
}

export function useCache() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.cache, queryFn: cache.info });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.cache);
    return {
      clear: () => done(cache.clear()),
    };
  }, [queryClient]);
  return { ...query, ...actions };
}

export function useAppConfig() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.config, queryFn: config.list });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.config);
    return {
      set: (key: string, value: unknown) => done(config.set(key, value)),
    };
  }, [queryClient]);
  return { ...query, ...actions };
}

export function useSyncSettings() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.sync, queryFn: sync.get });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.sync);
    return {
      set: (kind: SyncKind, targets: SyncTargets) =>
        done(sync.set(kind, targets)),
    };
  }, [queryClient]);
  return { ...query, ...actions };
}
