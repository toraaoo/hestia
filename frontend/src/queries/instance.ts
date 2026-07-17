/**
 * `instance.*` — queries/mutations plus their 1:1 hooks, keyed by stable id
 * like the server hooks. There is no `instance.status` channel, so
 * `useInstance` selects the entry from the list query. `create`/`update` are
 * plain (long) calls, not jobs; `launch` is the job that materialises files
 * and spawns the game.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type {
  ConfigEntry,
  ContentAddSpec,
  ContentDone,
  ContentKind,
  ContentProfile,
  InstanceCreateParams,
  InstanceInfo,
  InstanceLaunchDone,
  InstanceLaunchParams,
  InstanceUpdateParams,
  ResolveParams,
} from '../api';
import * as api from '../api/instance';
import { mutation } from './core';
import { jobMutation, useJobMutation } from './jobs';
import { keys } from './keys';
import { type LogsOptions, type LogsResult, useFollowedLogs } from './logs';

const CATALOG_STALE_MS = 5 * 60_000;

export const instanceQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.instances.list(),
      queryFn: () => api.list(),
    }),
  flavors: () =>
    queryOptions({
      queryKey: keys.instances.flavors(),
      queryFn: () => api.flavors(),
      staleTime: CATALOG_STALE_MS,
    }),
  versions: (flavor: string) =>
    queryOptions({
      queryKey: keys.instances.versions(flavor),
      queryFn: () => api.versions(flavor),
      staleTime: CATALOG_STALE_MS,
    }),
  profile: (params: ResolveParams) =>
    queryOptions({
      queryKey: keys.instances.profile(params),
      queryFn: () => api.resolve(params),
      staleTime: CATALOG_STALE_MS,
    }),
  /** Save-world folder names, for the datapack world picker. */
  worlds: (id: string) =>
    queryOptions({
      queryKey: keys.instances.worlds(id),
      queryFn: () => api.worlds(id),
    }),
  logs: (id: string, options: { session?: string; tail?: number } = {}) =>
    queryOptions({
      queryKey: keys.instances.logs(id, options.session, options.tail),
      queryFn: () => api.logs(id, options),
    }),
  config: (id: string) =>
    queryOptions({
      queryKey: keys.instances.config(id),
      queryFn: () => api.config.list(id),
    }),
  configValue: (id: string, key: string) =>
    queryOptions({
      queryKey: keys.instances.configValue(id, key),
      queryFn: () => api.config.get(id, key),
    }),
  content: (id: string, kind: ContentKind) =>
    queryOptions({
      queryKey: keys.instances.contentList(id, kind),
      queryFn: () => api.content.list(id, kind),
    }),
  /** The active profile name and every content profile of the instance. */
  profiles: (id: string) =>
    queryOptions({
      queryKey: keys.instances.profiles(id),
      queryFn: () => api.profiles.list(id),
    }),
};

export const instanceMutations = {
  create: () =>
    mutation<InstanceInfo, InstanceCreateParams>({
      mutationKey: [...keys.instances.all, 'create'],
      mutationFn: (params) => api.create(params),
      invalidates: () => [keys.instances.all],
    }),
  /** The instance pays for the new version at its next launch. */
  update: (id: string) =>
    mutation<InstanceInfo, Omit<InstanceUpdateParams, 'instance'>>({
      mutationKey: [...keys.instances.detail(id), 'update'],
      mutationFn: (params) => api.update({ ...params, instance: id }),
      invalidates: () => [keys.instances.all],
    }),
  rename: (id: string) =>
    mutation<InstanceInfo, string>({
      mutationKey: [...keys.instances.detail(id), 'rename'],
      mutationFn: (name) => api.rename(id, name),
      invalidates: () => [keys.instances.all],
    }),
  remove: (id: string) =>
    mutation({
      mutationKey: [...keys.instances.detail(id), 'remove'],
      mutationFn: () => api.remove(id),
      invalidates: () => [keys.instances.all, keys.processes.all],
    }),
  /** Materialise files and spawn the game as the signed-in account. */
  launch: (id: string) =>
    jobMutation<InstanceLaunchDone, Omit<InstanceLaunchParams, 'instance'>>({
      mutationKey: [...keys.instances.detail(id), 'launch'],
      meta: () => ({
        kind: 'instance.launch',
        label: 'launch',
        entry: { kind: 'instance', id },
      }),
      run: (params, onProgress) =>
        api.launch({ ...params, instance: id }, onProgress),
      invalidates: () => [keys.instances.all, keys.processes.all],
    }),
  /** Stops one named session, or every session of the instance. */
  stop: (id: string) =>
    mutation<void, { session?: string }>({
      mutationKey: [...keys.instances.detail(id), 'stop'],
      mutationFn: ({ session }) => api.stop(id, session),
      invalidates: () => [keys.instances.all, keys.processes.all],
    }),
  /** `memory` and `jvm-args` only. */
  setConfig: (id: string) =>
    mutation<void, ConfigEntry>({
      mutationKey: [...keys.instances.detail(id), 'config', 'set'],
      mutationFn: ({ key, value }) => api.config.set(id, key, value),
      invalidates: () => [keys.instances.config(id)],
    }),
  profiles: {
    /** Seeded with every selectable pool item unless `seedFromPool` is false. */
    create: (id: string) =>
      mutation<ContentProfile, { name: string; seedFromPool?: boolean }>({
        mutationKey: [...keys.instances.profiles(id), 'create'],
        mutationFn: ({ name, seedFromPool }) =>
          api.profiles.create(id, name, seedFromPool),
        invalidates: () => [keys.instances.profiles(id)],
      }),
    /** Removing the active profile clears the active selection. */
    remove: (id: string) =>
      mutation<void, string>({
        mutationKey: [...keys.instances.profiles(id), 'remove'],
        mutationFn: (name) => api.profiles.remove(id, name),
        invalidates: () => [keys.instances.profiles(id)],
      }),
    rename: (id: string) =>
      mutation<ContentProfile, { name: string; newName: string }>({
        mutationKey: [...keys.instances.profiles(id), 'rename'],
        mutationFn: ({ name, newName }) =>
          api.profiles.rename(id, name, newName),
        invalidates: () => [keys.instances.profiles(id)],
      }),
    /** Sets the active profile (empty clears it); applied at the next launch. */
    use: (id: string) =>
      mutation<void, string>({
        mutationKey: [...keys.instances.profiles(id), 'use'],
        mutationFn: (name) => api.profiles.use(id, name),
        invalidates: () => [keys.instances.profiles(id)],
      }),
    /** Add/remove members by pool reference. */
    edit: (id: string) =>
      mutation<
        ContentProfile,
        { name: string; add?: string[]; remove?: string[] }
      >({
        mutationKey: [...keys.instances.profiles(id), 'edit'],
        mutationFn: ({ name, add, remove }) =>
          api.profiles.edit(id, name, add, remove),
        invalidates: () => [keys.instances.profiles(id)],
      }),
  },
  content: {
    /** Instances take mods, resourcepacks, shaders, and datapacks. */
    add: (id: string) =>
      jobMutation<ContentDone, ContentAddSpec>({
        mutationKey: [...keys.instances.content(id), 'add'],
        meta: (spec) => ({
          kind: 'content.add',
          label: `add ${spec.kind}`,
          entry: { kind: 'instance', id },
        }),
        run: (spec, onProgress) => api.content.add(id, spec, onProgress),
        invalidates: () => [keys.instances.content(id)],
      }),
    remove: (id: string) =>
      mutation<void, { kind: ContentKind; item: string; worlds?: string[] }>({
        mutationKey: [...keys.instances.content(id), 'remove'],
        mutationFn: ({ kind, item, worlds }) =>
          api.content.remove(id, kind, item, worlds),
        // A removal also drops the filename from every profile.
        invalidates: () => [
          keys.instances.content(id),
          keys.instances.profiles(id),
        ],
      }),
    /** `item` empty updates every platform-sourced item of the kind. */
    update: (id: string) =>
      jobMutation<ContentDone, { kind: ContentKind; item?: string }>({
        mutationKey: [...keys.instances.content(id), 'update'],
        meta: ({ kind }) => ({
          kind: 'content.update',
          label: `update ${kind}s`,
          entry: { kind: 'instance', id },
        }),
        run: ({ kind, item }, onProgress) =>
          api.content.update(id, kind, item, onProgress),
        // An update can change a member's filename in every profile.
        invalidates: () => [
          keys.instances.content(id),
          keys.instances.profiles(id),
        ],
      }),
  },
};

export function useInstances() {
  return useQuery(instanceQueries.list());
}

/** One instance, selected out of the list query (there is no status channel). */
export function useInstance(id: string) {
  return useQuery({
    ...instanceQueries.list(),
    select: (instances: InstanceInfo[]) =>
      instances.find((instance) => instance.id === id) ?? null,
  });
}

export function useInstanceFlavors() {
  return useQuery(instanceQueries.flavors());
}

export function useInstanceVersions(flavor: string) {
  return useQuery(instanceQueries.versions(flavor));
}

export function useInstanceProfile(params: ResolveParams) {
  return useQuery(instanceQueries.profile(params));
}

export function useInstanceWorlds(id: string) {
  return useQuery(instanceQueries.worlds(id));
}

/** Follows the named session, or every session of the instance. */
export function useInstanceLogs(
  id: string,
  options: LogsOptions & { session?: string } = {},
): LogsResult {
  const query = useQuery({
    ...instanceQueries.logs(id, {
      session: options.session,
      tail: options.tail,
    }),
    staleTime: options.follow ? Number.POSITIVE_INFINITY : undefined,
  });
  const session = options.session;
  return useFollowedLogs(
    query,
    options.follow
      ? (processId) =>
          session
            ? processId === session
            : processId.startsWith(`instance-${id}_`)
      : null,
    options.limit,
  );
}

export function useInstanceConfig(id: string) {
  return useQuery(instanceQueries.config(id));
}

export function useInstanceConfigValue(id: string, key: string) {
  return useQuery(instanceQueries.configValue(id, key));
}

export function useInstanceContent(id: string, kind: ContentKind) {
  return useQuery(instanceQueries.content(id, kind));
}

export function useCreateInstance() {
  return useMutation(instanceMutations.create());
}

export function useUpdateInstance(id: string) {
  return useMutation(instanceMutations.update(id));
}

export function useRenameInstance(id: string) {
  return useMutation(instanceMutations.rename(id));
}

export function useRemoveInstance(id: string) {
  return useMutation(instanceMutations.remove(id));
}

export function useLaunchInstance(id: string) {
  return useJobMutation(instanceMutations.launch(id));
}

export function useStopInstance(id: string) {
  return useMutation(instanceMutations.stop(id));
}

export function useSetInstanceConfig(id: string) {
  return useMutation(instanceMutations.setConfig(id));
}

export function useInstanceProfiles(id: string) {
  return useQuery(instanceQueries.profiles(id));
}

export function useCreateInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.create(id));
}

export function useRemoveInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.remove(id));
}

export function useRenameInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.rename(id));
}

/** Sets the active profile (empty clears it); applied at the next launch. */
export function useUseInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.use(id));
}

export function useEditInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.edit(id));
}

export function useAddInstanceContent(id: string) {
  return useJobMutation(instanceMutations.content.add(id));
}

export function useRemoveInstanceContent(id: string) {
  return useMutation(instanceMutations.content.remove(id));
}

export function useUpdateInstanceContent(id: string) {
  return useJobMutation(instanceMutations.content.update(id));
}
