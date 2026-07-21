import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import type {
  ContentKind,
  GlobalProfile,
  InstanceInfo,
  ServerInfo,
} from '@/api';
import { m } from '@/paraglide/messages.js';
import { instanceQueries, useInstances } from '@/queries/instance';
import { useGlobalProfiles } from '@/queries/profile';
import { serverQueries, useServers } from '@/queries/server';

/** An entry the content can be installed into, drawn from every store. */
export interface Target {
  id: string;
  name: string;
  type: 'server' | 'instance' | 'profile';
  flavor: string;
  gameVersion: string;
  running: boolean;
}

export const serverTarget = (s: ServerInfo): Target => ({
  id: s.id,
  name: s.name,
  type: 'server',
  flavor: s.flavor,
  gameVersion: s.gameVersion,
  running: s.process?.state === 'running',
});

export const instanceTarget = (i: InstanceInfo): Target => ({
  id: i.id,
  name: i.name,
  type: 'instance',
  flavor: i.flavor,
  gameVersion: i.gameVersion,
  running: (i.sessions ?? []).some((s) => s.state === 'running'),
});

/**
 * A global profile as an install target: references, never jars — a profile
 * has no version or loader of its own, so anything compatible can join it.
 */
export const profileTarget = (p: GlobalProfile): Target => ({
  id: p.name,
  name: p.name,
  type: 'profile',
  flavor: '',
  gameVersion: '',
  running: false,
});

/** Which kinds each entry type accepts — mirrors the daemon's install surface. */
export const ACCEPTS: Record<Target['type'], ContentKind[]> = {
  profile: ['mod', 'resource_pack', 'shader'],
  server: ['mod', 'data_pack'],
  instance: ['mod', 'resource_pack', 'shader', 'data_pack'],
};

/** A mod needs a loader; a vanilla entry cannot take one. */
export const targetTakesKind = (t: Target, kind: ContentKind): boolean =>
  ACCEPTS[t.type].includes(kind) &&
  (kind !== 'mod' || t.flavor === 'fabric' || t.type === 'profile');

export const entryTypeLabel = (type: Target['type']): string =>
  type === 'server'
    ? m['entry.type_server']()
    : type === 'profile'
      ? m['entry.type_profile']()
      : m['entry.type_instance']();

/** A local file staged for import, tagged with the kind it installs as. */
export interface PickedFile {
  path: string;
  kind: ContentKind;
}

export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Every entry, from all three stores, merged into a common target shape. */
export function useTargets(): Target[] {
  const servers = useServers();
  const instances = useInstances();
  const profiles = useGlobalProfiles();
  return useMemo(
    () => [
      ...(servers.data ?? []).map(serverTarget),
      ...(instances.data ?? []).map(instanceTarget),
      ...(profiles.data ?? []).map(profileTarget),
    ],
    [servers.data, instances.data, profiles.data],
  );
}

/**
 * The installed pool of a target, keyed `source:projectId` — the same match the
 * CLI's browse session uses to flag an already-installed hit. Built on the
 * server/instance content-list factories; a profile holds references, not an
 * installable pool, so it reports nothing.
 */
export function useInstalledRefs(
  target: Target,
  kind: ContentKind,
): Set<string> {
  const server = useQuery({
    ...serverQueries.content(target.id, kind),
    enabled: target.type === 'server',
  });
  const instance = useQuery({
    ...instanceQueries.content(target.id, kind),
    enabled: target.type === 'instance',
  });
  const items = (target.type === 'server' ? server : instance).data?.items;
  return useMemo(
    () =>
      new Set(
        (items ?? [])
          .filter((i) => i.projectId)
          .map((i) => `${i.source}:${i.projectId}`),
      ),
    [items],
  );
}
