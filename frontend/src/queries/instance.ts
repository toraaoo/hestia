/**
 * `instance.*` — queries/mutations plus their 1:1 hooks, keyed by stable id
 * like the server hooks. There is no `instance.status` channel, so
 * `useInstance` selects the entry from the list query. `create`/`update` are
 * plain (long) calls, not jobs; `launch` is the job that materialises files
 * and spawns the game.
 */
import { queryOptions, useQuery } from '@tanstack/react-query';
import type {
  ConfigEntry,
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
import { CATALOG_STALE_MS, mutation } from './core';
import { entryContentFactories } from './entry-content';
import { useEntryIconLookup } from './icons';
import { jobMutation } from './jobs';
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
  /**
   * Instances take mods, resourcepacks, shaders, and datapacks. A
   * remove/update/pin can drop or remap a member's filename in every profile,
   * so the instance also sweeps its `profiles(id)` key.
   */
  content: entryContentFactories({
    kind: 'instance',
    api: api.content,
    contentKey: keys.instances.content,
    infoKey: keys.instances.info,
    extraInvalidate: (id) => [keys.instances.profiles(id)],
  }),
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
