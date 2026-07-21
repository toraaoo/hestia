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
  InstanceUpdateParams,
  ResolveParams,
} from '../api';
import * as api from '../api/instance';
import { CATALOG_STALE_MS, mutation, type QueryFlags } from './core';
import { useEntryIconLookup } from './icons';
import { jobMutation, useJobMutation } from './jobs';
import { keys } from './keys';
import { type LogsOptions, type LogsResult, useFollowedLogs } from './logs';

export const instanceQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.instances.list(),
      queryFn: () => api.list(),
    }),
  // The informational view (locations + footprint) is a directory walk, so it
  // rides its own key — fetched fresh (never seeded from the diskless list).
  info: (id: string) =>
    queryOptions({
      queryKey: keys.instances.info(id),
      queryFn: () => api.info(id),
      staleTime: 60_000,
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
  loaders: (flavor: string, version: string) =>
    queryOptions({
      queryKey: keys.instances.loaders(flavor, version),
      queryFn: () => api.loaders(flavor, version),
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
  contentUpdates: (id: string, kind: ContentKind) =>
    queryOptions({
      queryKey: keys.instances.contentUpdates(id, kind),
      queryFn: () => api.content.checkUpdates(id, kind),
      // A network resolve per item — refetch only when explicitly asked.
      staleTime: Number.POSITIVE_INFINITY,
      enabled: false,
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
      invalidates: () => [keys.instances.list()],
    }),
  /** The instance pays for the new version at its next launch. */
  update: (id: string) =>
    mutation<InstanceInfo, Omit<InstanceUpdateParams, 'instance'>>({
      mutationKey: [...keys.instances.detail(id), 'update'],
      mutationFn: (params) => api.update({ ...params, instance: id }),
      invalidates: () => [
        keys.instances.list(),
        keys.instances.detail(id),
        keys.instances.info(id),
      ],
    }),
  rename: (id: string) =>
    mutation<InstanceInfo, string>({
      mutationKey: [...keys.instances.detail(id), 'rename'],
      mutationFn: (name) => api.rename(id, name),
      invalidates: () => [keys.instances.list(), keys.instances.detail(id)],
    }),
  remove: (id: string) =>
    mutation({
      mutationKey: [...keys.instances.detail(id), 'remove'],
      mutationFn: () => api.remove(id),
      invalidates: () => [keys.instances.list(), keys.processes.list()],
    }),
  /** Stops one named session, or every session of the instance. */
  stop: (id: string) =>
    mutation<void, { session?: string }>({
      mutationKey: [...keys.instances.detail(id), 'stop'],
      mutationFn: ({ session }) => api.stop(id, session),
      invalidates: () => [
        keys.instances.list(),
        keys.instances.detail(id),
        keys.processes.list(),
      ],
    }),
  /**
   * Id-by-variable variants for list rows, which can't call a per-id hook. A
   * launch materialises files, so it streams provisioning progress through the
   * job store like the per-id `launch` above.
   */
  launchAny: () =>
    jobMutation<InstanceLaunchDone, string>({
      mutationKey: [...keys.instances.all, 'launch'],
      meta: (id) => ({
        kind: 'instance.launch',
        label: 'launch',
        entry: { kind: 'instance', id },
      }),
      run: (id, onProgress) => api.launch({ instance: id }, onProgress),
      invalidates: (id) => [
        keys.instances.list(),
        keys.instances.detail(id),
        keys.processes.list(),
      ],
    }),
  stopAny: () =>
    mutation<void, string>({
      mutationKey: [...keys.instances.all, 'stop'],
      mutationFn: (id) => api.stop(id),
      invalidates: (id) => [
        keys.instances.list(),
        keys.instances.detail(id),
        keys.processes.list(),
      ],
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
    /** Capture the profile's settings store; the instance must be stopped. */
    capture: (id: string) =>
      mutation<void, string>({
        mutationKey: [...keys.instances.profiles(id), 'capture'],
        mutationFn: (name) => api.profiles.capture(id, name),
        invalidates: () => [keys.instances.profiles(id)],
      }),
    /** Release the captured store; the instance must be stopped. */
    release: (id: string) =>
      mutation<void, string>({
        mutationKey: [...keys.instances.profiles(id), 'release'],
        mutationFn: (name) => api.profiles.release(id, name),
        invalidates: () => [keys.instances.profiles(id)],
      }),
    /**
     * Apply a global profile into the pool — a content job; installs are
     * tagged with the profile and never removed by a later apply.
     */
    apply: (id: string) =>
      jobMutation<ContentDone, string>({
        mutationKey: [...keys.instances.content(id), 'profile-apply'],
        meta: (profile) => ({
          kind: 'profile.apply',
          label: `apply ${profile}`,
          entry: { kind: 'instance', id },
        }),
        run: (profile, onProgress) =>
          api.profiles.apply(id, profile, onProgress),
        invalidates: () => [
          keys.instances.content(id),
          keys.instances.profiles(id),
          keys.instances.info(id),
        ],
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
        invalidates: () => [
          keys.instances.content(id),
          keys.instances.info(id),
        ],
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
          keys.instances.info(id),
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
          keys.instances.info(id),
        ],
      }),
    enable: (id: string) =>
      mutation<
        void,
        { kind: ContentKind; item: string; enabled: boolean; worlds?: string[] }
      >({
        mutationKey: [...keys.instances.content(id), 'enable'],
        mutationFn: ({ kind, item, enabled, worlds }) =>
          api.content.enable(id, kind, item, enabled, worlds),
        invalidates: () => [
          keys.instances.content(id),
          keys.instances.info(id),
        ],
      }),
    setVersion: (id: string) =>
      jobMutation<
        ContentDone,
        { kind: ContentKind; item: string; version: string }
      >({
        mutationKey: [...keys.instances.content(id), 'set-version'],
        meta: ({ kind }) => ({
          kind: 'content.update',
          label: `pin ${kind}`,
          entry: { kind: 'instance', id },
        }),
        run: ({ kind, item, version }, onProgress) =>
          api.content.setVersion(id, kind, item, version, onProgress),
        // A pin can change a member's filename in every profile.
        invalidates: () => [
          keys.instances.content(id),
          keys.instances.profiles(id),
          keys.instances.info(id),
        ],
      }),
  },
};

export function useInstances() {
  const iconFor = useEntryIconLookup();
  return useQuery({
    ...instanceQueries.list(),
    select: (instances) =>
      instances.map((instance) => ({
        ...instance,
        iconUrl: iconFor(instance.id),
      })),
  });
}

/** The instance's static, informational view (locations + disk footprint). */
export function useInstanceInfo(id: string) {
  return useQuery(instanceQueries.info(id));
}

export function useLaunchInstanceAny() {
  return useJobMutation(instanceMutations.launchAny());
}

export function useStopInstanceAny() {
  return useMutation(instanceMutations.stopAny());
}

/** One instance, selected out of the list query (there is no status channel). */
export function useInstance(id: string) {
  const iconFor = useEntryIconLookup();
  return useQuery({
    ...instanceQueries.list(),
    select: (instances: InstanceInfo[]) => {
      const instance = instances.find((entry) => entry.id === id);
      return instance ? { ...instance, iconUrl: iconFor(instance.id) } : null;
    },
  });
}

export function useInstanceFlavors({ enabled = true }: QueryFlags = {}) {
  return useQuery({ ...instanceQueries.flavors(), enabled });
}

export function useInstanceVersions(
  flavor: string,
  { enabled = true }: QueryFlags = {},
) {
  return useQuery({ ...instanceQueries.versions(flavor), enabled });
}

export function useInstanceLoaders(
  flavor: string,
  version: string,
  { enabled = true }: QueryFlags = {},
) {
  return useQuery({
    ...instanceQueries.loaders(flavor, version),
    enabled: enabled && flavor !== '' && version !== '',
  });
}

export function useInstanceProfile(params: ResolveParams) {
  return useQuery(instanceQueries.profile(params));
}

export function useInstanceWorlds(
  id: string,
  { enabled = true }: QueryFlags = {},
) {
  return useQuery({ ...instanceQueries.worlds(id), enabled });
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

export function useInstanceContent(
  id: string,
  kind: ContentKind,
  { enabled = true }: QueryFlags = {},
) {
  return useQuery({ ...instanceQueries.content(id, kind), enabled });
}

/** Update availability — disabled until `refetch()` is called (network per item). */
export function useInstanceContentUpdates(id: string, kind: ContentKind) {
  return useQuery(instanceQueries.contentUpdates(id, kind));
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

/** Capture the profile's settings store; the instance must be stopped. */
export function useCaptureInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.capture(id));
}

/** Release the captured store; the instance must be stopped. */
export function useReleaseInstanceProfile(id: string) {
  return useMutation(instanceMutations.profiles.release(id));
}

/** Apply a global profile into the instance's pool (a content job). */
export function useApplyInstanceProfile(id: string) {
  return useJobMutation(instanceMutations.profiles.apply(id));
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

export function useEnableInstanceContent(id: string) {
  return useMutation(instanceMutations.content.enable(id));
}

export function useSetInstanceContentVersion(id: string) {
  return useJobMutation(instanceMutations.content.setVersion(id));
}
