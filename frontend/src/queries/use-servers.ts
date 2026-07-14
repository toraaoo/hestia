/**
 * Entity-scoped server hooks: `useServer(id)` is the status query spread
 * together with every action bound to that server; `useServers()` is the
 * list plus `create`. Entries are referenced by their stable id (the wire
 * also resolves display names, but the frontend keys everything by id so a
 * rename cannot strand a cache entry). Actions are plain async functions
 * that invalidate on settle — lifecycle verbs sweep the whole `servers`
 * prefix, sub-resource verbs only the entry's own `detail` subtree.
 * Sub-resources (logs, config, backups, content) keep their own query hooks
 * so a card that only shows a name and a play button doesn't fetch them.
 */
import {
  type QueryClient,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import { useMemo } from 'react';
import type {
  ContentAddSpec,
  ContentKind,
  ProvisionProgress,
  ResolveParams,
  ServerCreateParams,
  ServerInfo,
  ServerUpdateParams,
} from '../api';
import { server } from '../api';
import { sweeper } from './client';
import { keys } from './keys';

type OnProgress = (progress: ProvisionProgress) => void;

function makeServerActions(id: string, queryClient: QueryClient) {
  const swept = sweeper(queryClient, keys.servers.all);
  const entry = sweeper(queryClient, keys.servers.detail(id));
  return {
    start: () => swept(server.start(id)),
    stop: () => swept(server.stop(id)),
    remove: () => swept(server.remove(id)),
    rename: (to: string) => swept(server.rename(id, to)),
    update: (
      params: Omit<ServerUpdateParams, 'server'>,
      onProgress?: OnProgress,
    ) => swept(server.update({ ...params, server: id }, onProgress)),
    /** One console command over RCON; touches no cached state. */
    command: (line: string) => server.command(id, line),
    getConfig: (key: string) => server.config.get(id, key),
    setConfig: (key: string, value: string) =>
      entry(server.config.set(id, key, value)),
    backup: {
      create: (onProgress?: OnProgress) =>
        entry(server.backup.create(id, onProgress)),
      restore: (backupId: string, onProgress?: OnProgress) =>
        entry(server.backup.restore(id, backupId, onProgress)),
      remove: (backupId: string) => entry(server.backup.remove(id, backupId)),
    },
    content: {
      add: (spec: ContentAddSpec, onProgress?: OnProgress) =>
        entry(server.content.add(id, spec, onProgress)),
      remove: (kind: ContentKind, item: string, worlds?: string[]) =>
        entry(server.content.remove(id, kind, item, worlds)),
      update: (kind: ContentKind, item?: string, onProgress?: OnProgress) =>
        entry(server.content.update(id, kind, item, onProgress)),
    },
  };
}

export type ServerActions = ReturnType<typeof makeServerActions>;

/** The actions alone — for call sites that already have the entity's data. */
export function useServerActions(id: string): ServerActions {
  const queryClient = useQueryClient();
  return useMemo(() => makeServerActions(id, queryClient), [id, queryClient]);
}

/**
 * One server: its status query plus its bound actions. Seeds from the list
 * cache, so rendering a row of an already-fetched list costs no extra call.
 */
export function useServer(id: string) {
  const queryClient = useQueryClient();
  const actions = useServerActions(id);
  const query = useQuery({
    queryKey: keys.servers.detail(id),
    queryFn: () => server.status(id),
    enabled: id.length > 0,
    initialData: () =>
      queryClient
        .getQueryData<ServerInfo[]>(keys.servers.list)
        ?.find((s) => s.id === id),
    initialDataUpdatedAt: () =>
      queryClient.getQueryState(keys.servers.list)?.dataUpdatedAt,
  });
  return { ...query, ...actions };
}

/** Every server, plus the collection-level `create`. */
export function useServers() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.servers.list, queryFn: server.list });
  const create = useMemo(() => {
    const swept = sweeper(queryClient, keys.servers.all);
    return (params: ServerCreateParams, onProgress?: OnProgress) =>
      swept(server.create(params, onProgress));
  }, [queryClient]);
  return { ...query, create };
}

export function useServerFlavors() {
  return useQuery({
    queryKey: keys.servers.flavors,
    queryFn: server.flavors,
    staleTime: 60 * 60_000,
  });
}

export function useServerVersions(flavor: string) {
  return useQuery({
    queryKey: keys.servers.versions(flavor),
    queryFn: () => server.versions(flavor),
    enabled: flavor.length > 0,
    staleTime: 10 * 60_000,
  });
}

export function useServerResolve(params: ResolveParams, enabled = true) {
  return useQuery({
    queryKey: [...keys.servers.all, 'resolve', params] as const,
    queryFn: () => server.resolve(params),
    enabled: enabled && params.flavor.length > 0 && params.version.length > 0,
    staleTime: 10 * 60_000,
  });
}

export function useServerLogs(id: string, tail?: number) {
  return useQuery({
    queryKey: keys.servers.logs(id, tail),
    queryFn: () => server.logs(id, tail),
    enabled: id.length > 0,
  });
}

export function useServerConfig(id: string) {
  return useQuery({
    queryKey: keys.servers.config(id),
    queryFn: () => server.config.list(id),
    enabled: id.length > 0,
  });
}

export function useServerBackups(id: string) {
  return useQuery({
    queryKey: keys.servers.backups(id),
    queryFn: () => server.backup.list(id),
    enabled: id.length > 0,
  });
}

export function useServerContent(id: string, kind: ContentKind) {
  return useQuery({
    queryKey: keys.servers.content(id, kind),
    queryFn: () => server.content.list(id, kind),
    enabled: id.length > 0,
  });
}
