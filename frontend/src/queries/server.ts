/**
 * `server.*` — queries/mutations plus their 1:1 hooks. Per-entry factories
 * and hooks take the server's **stable id**; the wire resolves display names
 * too, but keying by id means a rename never strands a cache entry or a
 * mutation key.
 */
import {
  queryOptions,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import type {
  BackupInfo,
  ConfigEntry,
  ContentAddSpec,
  ContentDone,
  ContentKind,
  ResolveParams,
  ServerCreateParams,
  ServerInfo,
  ServerUpdateParams,
} from '../api';
import * as api from '../api/server';
import { mutation } from './core';
import { jobMutation, useJobMutation } from './jobs';
import { keys } from './keys';
import { type LogsOptions, type LogsResult, useFollowedLogs } from './logs';

const CATALOG_STALE_MS = 5 * 60_000;

export const serverQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.servers.list(),
      queryFn: () => api.list(),
    }),
  detail: (id: string) =>
    queryOptions({
      queryKey: keys.servers.detail(id),
      queryFn: () => api.status(id),
    }),
  // The informational view (locations + footprint) is a directory walk, so it
  // rides its own key — fetched fresh (never seeded from the diskless list) and
  // off the frequently-invalidated detail query.
  info: (id: string) =>
    queryOptions({
      queryKey: keys.servers.info(id),
      queryFn: () => api.info(id),
      staleTime: 60_000,
    }),
  ping: (id: string) =>
    queryOptions({
      queryKey: keys.servers.ping(id),
      queryFn: () => api.ping(id),
      meta: { silent: true },
    }),
  flavors: () =>
    queryOptions({
      queryKey: keys.servers.flavors(),
      queryFn: () => api.flavors(),
      staleTime: CATALOG_STALE_MS,
    }),
  versions: (flavor: string) =>
    queryOptions({
      queryKey: keys.servers.versions(flavor),
      queryFn: () => api.versions(flavor),
      staleTime: CATALOG_STALE_MS,
    }),
  loaders: (flavor: string, version: string) =>
    queryOptions({
      queryKey: keys.servers.loaders(flavor, version),
      queryFn: () => api.loaders(flavor, version),
      staleTime: CATALOG_STALE_MS,
    }),
  profile: (params: ResolveParams) =>
    queryOptions({
      queryKey: keys.servers.profile(params),
      queryFn: () => api.resolve(params),
      staleTime: CATALOG_STALE_MS,
    }),
  logs: (id: string, tail?: number) =>
    queryOptions({
      queryKey: keys.servers.logs(id, tail),
      queryFn: () => api.logs(id, tail),
    }),
  config: (id: string) =>
    queryOptions({
      queryKey: keys.servers.config(id),
      queryFn: () => api.config.list(id),
    }),
  configValue: (id: string, key: string) =>
    queryOptions({
      queryKey: keys.servers.configValue(id, key),
      queryFn: () => api.config.get(id, key),
    }),
  backups: (id: string) =>
    queryOptions({
      queryKey: keys.servers.backups(id),
      queryFn: () => api.backup.list(id),
    }),
  content: (id: string, kind: ContentKind) =>
    queryOptions({
      queryKey: keys.servers.contentList(id, kind),
      queryFn: () => api.content.list(id, kind),
    }),
  contentUpdates: (id: string, kind: ContentKind) =>
    queryOptions({
      queryKey: keys.servers.contentUpdates(id, kind),
      queryFn: () => api.content.checkUpdates(id, kind),
      // A network resolve per item — refetch only when explicitly asked.
      staleTime: Number.POSITIVE_INFINITY,
      enabled: false,
    }),
};

export const serverMutations = {
  create: () =>
    jobMutation<ServerInfo, ServerCreateParams>({
      mutationKey: [...keys.servers.all, 'create'],
      meta: (params) => ({
        kind: 'server.create',
        label: `create ${params.name || `${params.flavor} ${params.version}`}`,
      }),
      run: (params, onProgress) => api.create(params, onProgress),
      invalidates: () => [keys.servers.list()],
    }),
  update: (id: string) =>
    jobMutation<ServerInfo, Omit<ServerUpdateParams, 'server'>>({
      mutationKey: [...keys.servers.detail(id), 'update'],
      meta: (params) => ({
        kind: 'server.update',
        label: `update to ${params.version}`,
        entry: { kind: 'server', id },
      }),
      run: (params, onProgress) =>
        api.update({ ...params, server: id }, onProgress),
      invalidates: () => [
        keys.servers.list(),
        keys.servers.detail(id),
        keys.servers.info(id),
      ],
    }),
  rename: (id: string) =>
    mutation<ServerInfo, string>({
      mutationKey: [...keys.servers.detail(id), 'rename'],
      mutationFn: (name) => api.rename(id, name),
      invalidates: () => [keys.servers.list(), keys.servers.detail(id)],
    }),
  remove: (id: string) =>
    mutation({
      mutationKey: [...keys.servers.detail(id), 'remove'],
      mutationFn: () => api.remove(id),
      invalidates: () => [keys.servers.list(), keys.processes.list()],
    }),
  start: (id: string) =>
    mutation<{ processId: string; pid: number }>({
      mutationKey: [...keys.servers.detail(id), 'start'],
      mutationFn: () => api.start(id),
      invalidates: () => [
        keys.servers.list(),
        keys.servers.detail(id),
        keys.processes.list(),
      ],
    }),
  stop: (id: string) =>
    mutation({
      mutationKey: [...keys.servers.detail(id), 'stop'],
      mutationFn: () => api.stop(id),
      invalidates: () => [
        keys.servers.list(),
        keys.servers.detail(id),
        keys.processes.list(),
      ],
    }),
  /** Id-by-variable variants for list rows, which can't call a per-id hook. */
  startAny: () =>
    mutation<{ processId: string; pid: number }, string>({
      mutationKey: [...keys.servers.all, 'start'],
      mutationFn: (id) => api.start(id),
      invalidates: (id) => [
        keys.servers.list(),
        keys.servers.detail(id),
        keys.processes.list(),
      ],
    }),
  stopAny: () =>
    mutation<void, string>({
      mutationKey: [...keys.servers.all, 'stop'],
      mutationFn: (id) => api.stop(id),
      invalidates: (id) => [
        keys.servers.list(),
        keys.servers.detail(id),
        keys.processes.list(),
      ],
    }),
  /** One console command over RCON; touches no cached state. */
  command: (id: string) =>
    mutation<string, string>({
      mutationKey: [...keys.servers.detail(id), 'command'],
      mutationFn: (line) => api.command(id, line),
    }),
  setConfig: (id: string) =>
    mutation<void, ConfigEntry>({
      mutationKey: [...keys.servers.detail(id), 'config', 'set'],
      mutationFn: ({ key, value }) => api.config.set(id, key, value),
      invalidates: () => [keys.servers.config(id)],
    }),
  backup: {
    /** Archives a running server live (world saving pauses over RCON). */
    create: (id: string) =>
      jobMutation<BackupInfo>({
        mutationKey: [...keys.servers.backups(id), 'create'],
        meta: () => ({
          kind: 'backup.create',
          label: 'back up',
          entry: { kind: 'server', id },
        }),
        run: (_variables, onProgress) => api.backup.create(id, onProgress),
        invalidates: () => [keys.servers.backups(id), keys.servers.info(id)],
      }),
    /** Refused while the server runs or is busy; swaps `data/` wholesale. */
    restore: (id: string) =>
      jobMutation<BackupInfo, string>({
        mutationKey: [...keys.servers.backups(id), 'restore'],
        meta: () => ({
          kind: 'backup.restore',
          label: 'restore backup',
          entry: { kind: 'server', id },
        }),
        run: (backupId, onProgress) =>
          api.backup.restore(id, backupId, onProgress),
        invalidates: () => [keys.servers.detail(id), keys.servers.info(id)],
      }),
    remove: (id: string) =>
      mutation<void, string>({
        mutationKey: [...keys.servers.backups(id), 'remove'],
        mutationFn: (backupId) => api.backup.remove(id, backupId),
        invalidates: () => [keys.servers.backups(id), keys.servers.info(id)],
      }),
  },
  content: {
    /** Servers take mods and datapacks; refused on a running or busy server. */
    add: (id: string) =>
      jobMutation<ContentDone, ContentAddSpec>({
        mutationKey: [...keys.servers.content(id), 'add'],
        meta: (spec) => ({
          kind: 'content.add',
          label: `add ${spec.kind}`,
          entry: { kind: 'server', id },
        }),
        run: (spec, onProgress) => api.content.add(id, spec, onProgress),
        invalidates: () => [keys.servers.content(id), keys.servers.info(id)],
      }),
    remove: (id: string) =>
      mutation<void, { kind: ContentKind; item: string; worlds?: string[] }>({
        mutationKey: [...keys.servers.content(id), 'remove'],
        mutationFn: ({ kind, item, worlds }) =>
          api.content.remove(id, kind, item, worlds),
        invalidates: () => [keys.servers.content(id), keys.servers.info(id)],
      }),
    /** `item` empty updates every platform-sourced item of the kind. */
    update: (id: string) =>
      jobMutation<ContentDone, { kind: ContentKind; item?: string }>({
        mutationKey: [...keys.servers.content(id), 'update'],
        meta: ({ kind }) => ({
          kind: 'content.update',
          label: `update ${kind}s`,
          entry: { kind: 'server', id },
        }),
        run: ({ kind, item }, onProgress) =>
          api.content.update(id, kind, item, onProgress),
        invalidates: () => [keys.servers.content(id), keys.servers.info(id)],
      }),
    enable: (id: string) =>
      mutation<
        void,
        { kind: ContentKind; item: string; enabled: boolean; worlds?: string[] }
      >({
        mutationKey: [...keys.servers.content(id), 'enable'],
        mutationFn: ({ kind, item, enabled, worlds }) =>
          api.content.enable(id, kind, item, enabled, worlds),
        invalidates: () => [keys.servers.content(id), keys.servers.info(id)],
      }),
    setVersion: (id: string) =>
      jobMutation<
        ContentDone,
        { kind: ContentKind; item: string; version: string }
      >({
        mutationKey: [...keys.servers.content(id), 'set-version'],
        meta: ({ kind }) => ({
          kind: 'content.update',
          label: `pin ${kind}`,
          entry: { kind: 'server', id },
        }),
        run: ({ kind, item, version }, onProgress) =>
          api.content.setVersion(id, kind, item, version, onProgress),
        invalidates: () => [keys.servers.content(id), keys.servers.info(id)],
      }),
  },
};

export function useServers() {
  return useQuery(serverQueries.list());
}

/** One server's status, seeded from the list cache so a row costs no call. */
export function useServer(id: string) {
  const queryClient = useQueryClient();
  return useQuery({
    ...serverQueries.detail(id),
    initialData: () =>
      queryClient
        .getQueryData<ServerInfo[]>(keys.servers.list())
        ?.find((server) => server.id === id),
    initialDataUpdatedAt: () =>
      queryClient.getQueryState(keys.servers.list())?.dataUpdatedAt,
  });
}

/**
 * The server's informational view (locations + footprint) — a directory walk,
 * on its own cadence.
 */
export function useServerInfo(id: string) {
  return useQuery(serverQueries.info(id));
}

/** A running server's live ping (players/MOTD); polls while `enabled`. */
export function useServerPing(id: string, enabled: boolean) {
  return useQuery({
    ...serverQueries.ping(id),
    enabled,
    refetchInterval: enabled ? 5000 : false,
    retry: false,
  });
}

export function useServerFlavors() {
  return useQuery(serverQueries.flavors());
}

export function useServerVersions(flavor: string) {
  return useQuery(serverQueries.versions(flavor));
}

export function useServerLoaders(flavor: string, version: string) {
  return useQuery({
    ...serverQueries.loaders(flavor, version),
    enabled: flavor !== '' && version !== '',
  });
}

export function useServerProfile(params: ResolveParams) {
  return useQuery(serverQueries.profile(params));
}

export function useServerLogs(
  id: string,
  options: LogsOptions = {},
): LogsResult {
  const query = useQuery({
    ...serverQueries.logs(id, options.tail),
    staleTime: options.follow ? Number.POSITIVE_INFINITY : undefined,
  });
  return useFollowedLogs(
    query,
    options.follow ? (processId) => processId === `server-${id}` : null,
    options.limit,
  );
}

export function useServerConfig(id: string) {
  return useQuery(serverQueries.config(id));
}

export function useServerConfigValue(id: string, key: string) {
  return useQuery(serverQueries.configValue(id, key));
}

export function useServerBackups(id: string) {
  return useQuery(serverQueries.backups(id));
}

export function useServerContent(id: string, kind: ContentKind) {
  return useQuery(serverQueries.content(id, kind));
}

/** Update availability — disabled until `refetch()` is called (network per item). */
export function useServerContentUpdates(id: string, kind: ContentKind) {
  return useQuery(serverQueries.contentUpdates(id, kind));
}

export function useCreateServer() {
  return useJobMutation(serverMutations.create());
}

export function useUpdateServer(id: string) {
  return useJobMutation(serverMutations.update(id));
}

export function useRenameServer(id: string) {
  return useMutation(serverMutations.rename(id));
}

export function useRemoveServer(id: string) {
  return useMutation(serverMutations.remove(id));
}

export function useStartServer(id: string) {
  return useMutation(serverMutations.start(id));
}

export function useStopServer(id: string) {
  return useMutation(serverMutations.stop(id));
}

export function useStartServerAny() {
  return useMutation(serverMutations.startAny());
}

export function useStopServerAny() {
  return useMutation(serverMutations.stopAny());
}

export function useServerCommand(id: string) {
  return useMutation(serverMutations.command(id));
}

export function useSetServerConfig(id: string) {
  return useMutation(serverMutations.setConfig(id));
}

export function useCreateServerBackup(id: string) {
  return useJobMutation(serverMutations.backup.create(id));
}

export function useRestoreServerBackup(id: string) {
  return useJobMutation(serverMutations.backup.restore(id));
}

export function useRemoveServerBackup(id: string) {
  return useMutation(serverMutations.backup.remove(id));
}

export function useAddServerContent(id: string) {
  return useJobMutation(serverMutations.content.add(id));
}

export function useRemoveServerContent(id: string) {
  return useMutation(serverMutations.content.remove(id));
}

export function useUpdateServerContent(id: string) {
  return useJobMutation(serverMutations.content.update(id));
}

export function useEnableServerContent(id: string) {
  return useMutation(serverMutations.content.enable(id));
}

export function useSetServerContentVersion(id: string) {
  return useJobMutation(serverMutations.content.setVersion(id));
}
