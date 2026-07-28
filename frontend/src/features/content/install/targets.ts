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
import { profileQueries } from '@/queries/profile';
import { serverQueries, useServers } from '@/queries/server';

/** An entry the content can be installed into, drawn from every store. */
export interface Target {
  id: string;
  name: string;
  type: 'server' | 'instance' | 'profile';
  flavor: string;
  gameVersion: string;
  running: boolean;
  /** What this entry takes, as the daemon computed it from its flavor. */
  accepts: ContentKind[];
}

export const serverTarget = (s: ServerInfo): Target => ({
  id: s.id,
  name: s.name,
  type: 'server',
  flavor: s.flavor,
  gameVersion: s.gameVersion,
  running: s.process?.state === 'running',
  accepts: s.accepts ?? [],
});

export const instanceTarget = (i: InstanceInfo): Target => ({
  id: i.id,
  name: i.name,
  type: 'instance',
  flavor: i.flavor,
  gameVersion: i.gameVersion,
  running: (i.sessions ?? []).some((s) => s.state === 'running'),
  accepts: i.accepts ?? [],
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
  accepts: PROFILE_ACCEPTS,
});

/**
 * A global profile stores project references rather than jars, so it has no
 * flavor to ask the daemon about — its kinds are the ones an instance can
 * later resolve a reference into. Servers and instances carry their own
 * `accepts` instead: what a paper server takes differs from what a fabric one
 * does, and only the daemon's flavor registry knows that.
 */
export const PROFILE_ACCEPTS: ContentKind[] = [
  'mod',
  'resource_pack',
  'shader',
];

export const targetTakesKind = (t: Target, kind: ContentKind): boolean =>
  t.accepts.includes(kind);

export const entryTypeLabel = (type: Target['type']): string =>
  type === 'server'
    ? m['entry.type_server']()
    : type === 'profile'
      ? m['entry.type_profile']()
      : m['entry.type_instance']();

/**
 * A local file staged for import, carrying the daemon's inspection. `kind` is
 * the effective kind (detected, then user-overridable); null means an
 * unrecognised-but-valid archive whose kind must be chosen. `valid` false marks
 * an un-installable file (modpack/corrupt) shown with `reason`.
 */
export interface PickedFile {
  path: string;
  filename: string;
  kind?: ContentKind;
  detected?: ContentKind;
  valid: boolean;
  reason: string;
}

export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Every entry, from all three stores, merged into a common target shape. */
export function useTargets(): Target[] {
  const servers = useServers();
  const instances = useInstances();
  const profiles = useQuery(profileQueries.list());
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
