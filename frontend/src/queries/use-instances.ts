/**
 * Entity-scoped instance hooks, the same shape as `use-servers.ts`:
 * `useInstance(id)` is the record (selected out of the shared list query —
 * instances have no standalone status channel) plus bound actions, keyed by
 * the stable id, never the display name. Lifecycle verbs sweep the whole
 * `instances` prefix; sub-resource verbs only the entry's `detail` subtree.
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
  InstanceCreateParams,
  InstanceLaunchParams,
  InstanceUpdateParams,
  ProvisionProgress,
  ResolveParams,
} from '../api';
import { instance } from '../api';
import { sweeper } from './client';
import { keys } from './keys';

type OnProgress = (progress: ProvisionProgress) => void;

function makeInstanceActions(id: string, queryClient: QueryClient) {
  const swept = sweeper(queryClient, keys.instances.all);
  const entry = sweeper(queryClient, keys.instances.detail(id));
  return {
    launch: (
      params: Omit<InstanceLaunchParams, 'instance'> = {},
      onProgress?: OnProgress,
    ) => swept(instance.launch({ ...params, instance: id }, onProgress)),
    /** Stops one named session, or every session of the instance. */
    stop: (session?: string) => swept(instance.stop(id, session)),
    remove: () => swept(instance.remove(id)),
    rename: (to: string) => swept(instance.rename(id, to)),
    update: (params: Omit<InstanceUpdateParams, 'instance'>) =>
      swept(instance.update({ ...params, instance: id })),
    getConfig: (key: string) => instance.config.get(id, key),
    setConfig: (key: string, value: string) =>
      entry(instance.config.set(id, key, value)),
    backup: {
      create: (onProgress?: OnProgress) =>
        entry(instance.backup.create(id, onProgress)),
      restore: (backupId: string, onProgress?: OnProgress) =>
        entry(instance.backup.restore(id, backupId, onProgress)),
      remove: (backupId: string) => entry(instance.backup.remove(id, backupId)),
    },
    content: {
      add: (spec: ContentAddSpec, onProgress?: OnProgress) =>
        entry(instance.content.add(id, spec, onProgress)),
      remove: (kind: ContentKind, item: string, worlds?: string[]) =>
        entry(instance.content.remove(id, kind, item, worlds)),
      update: (kind: ContentKind, item?: string, onProgress?: OnProgress) =>
        entry(instance.content.update(id, kind, item, onProgress)),
    },
  };
}

export type InstanceActions = ReturnType<typeof makeInstanceActions>;

/** The actions alone — for call sites that already have the entity's data. */
export function useInstanceActions(id: string): InstanceActions {
  const queryClient = useQueryClient();
  return useMemo(() => makeInstanceActions(id, queryClient), [id, queryClient]);
}

/**
 * One instance: its record (found in the shared list query — sessions
 * included, so N cards share one fetch) plus its bound actions.
 */
export function useInstance(id: string) {
  const actions = useInstanceActions(id);
  const query = useQuery({
    queryKey: keys.instances.list,
    queryFn: instance.list,
    enabled: id.length > 0,
    select: (instances) => instances.find((i) => i.id === id),
  });
  return { ...query, ...actions };
}

/** Every instance, plus the collection-level `create`. */
export function useInstances() {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: keys.instances.list,
    queryFn: instance.list,
  });
  const create = useMemo(() => {
    const swept = sweeper(queryClient, keys.instances.all);
    return (params: InstanceCreateParams) => swept(instance.create(params));
  }, [queryClient]);
  return { ...query, create };
}

export function useInstanceFlavors() {
  return useQuery({
    queryKey: keys.instances.flavors,
    queryFn: instance.flavors,
    staleTime: 60 * 60_000,
  });
}

export function useInstanceVersions(flavor: string) {
  return useQuery({
    queryKey: keys.instances.versions(flavor),
    queryFn: () => instance.versions(flavor),
    enabled: flavor.length > 0,
    staleTime: 10 * 60_000,
  });
}

export function useInstanceResolve(params: ResolveParams, enabled = true) {
  return useQuery({
    queryKey: [...keys.instances.all, 'resolve', params] as const,
    queryFn: () => instance.resolve(params),
    enabled: enabled && params.flavor.length > 0 && params.version.length > 0,
    staleTime: 10 * 60_000,
  });
}

export function useInstanceLogs(
  id: string,
  options: { session?: string; tail?: number } = {},
) {
  return useQuery({
    queryKey: keys.instances.logs(id, options.session, options.tail),
    queryFn: () => instance.logs(id, options),
    enabled: id.length > 0,
  });
}

export function useInstanceConfig(id: string) {
  return useQuery({
    queryKey: keys.instances.config(id),
    queryFn: () => instance.config.list(id),
    enabled: id.length > 0,
  });
}

export function useInstanceBackups(id: string) {
  return useQuery({
    queryKey: keys.instances.backups(id),
    queryFn: () => instance.backup.list(id),
    enabled: id.length > 0,
  });
}

export function useInstanceContent(id: string, kind: ContentKind) {
  return useQuery({
    queryKey: keys.instances.content(id, kind),
    queryFn: () => instance.content.list(id, kind),
    enabled: id.length > 0,
  });
}

/** Save-world folder names, for the datapack world picker. */
export function useInstanceWorlds(id: string) {
  return useQuery({
    queryKey: keys.instances.worlds(id),
    queryFn: () => instance.worlds(id),
    enabled: id.length > 0,
  });
}
