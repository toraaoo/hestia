/**
 * Event-driven cache freshness: daemon topics map to the key prefixes they
 * outdate, so lists stay live even for changes this window didn't make (the
 * CLI, the tray, a schedule). Mutations invalidate their own keys on settle
 * as well — this feed is the cross-client half. A reconnect invalidates
 * everything: the daemon may have changed while we were away.
 */
import type { QueryKey } from '@tanstack/react-query';
import { onConnectionChange, onDaemonEvent } from '../api';
import { invalidate, queryClient } from './client';
import { FOOTPRINT, keys } from './keys';

/** The entry id a supervisor process id names (`server-<id>` / `instance-<id>_<seq>`). */
function entryFromProcess(processId: string, prefix: string): string | null {
  if (!processId.startsWith(prefix)) return null;
  return processId.slice(prefix.length).split('_')[0] || null;
}

// Running state shows in both the entry's list row and its detail status, so a
// lifecycle event refreshes those two plus the process list — never the whole
// `all` subtree (other entries, the footprint walk).
function processKeys(payload: Record<string, unknown>): QueryKey[] {
  const id = String(payload.id ?? '');
  const server = entryFromProcess(id, 'server-');
  if (server)
    return [
      keys.processes.list(),
      keys.servers.list(),
      keys.servers.detail(server),
    ];
  const instance = entryFromProcess(id, 'instance-');
  if (instance)
    return [
      keys.processes.list(),
      keys.instances.list(),
      keys.instances.detail(instance),
    ];
  return [keys.processes.list()];
}

function serverEntry(payload: Record<string, unknown>): string | null {
  const server = payload.server as { id?: string } | undefined;
  return server?.id ?? null;
}

/** A list refresh, plus the named entry's detail when it is known. */
function entryKeys(
  kind: 'servers' | 'instances',
  id: string | null,
): QueryKey[] {
  const out: QueryKey[] = [keys[kind].list()];
  if (id) out.push(keys[kind].detail(id));
  return out;
}

const TOPICS: Record<string, (payload: Record<string, unknown>) => QueryKey[]> =
  {
    'process.started': processKeys,
    'process.exit': processKeys,
    'server.create.done': () => [keys.servers.list()],
    // A failed create removes the half-provisioned record.
    'server.create.error': () => [keys.servers.list()],
    'server.update.done': (p) => entryKeys('servers', serverEntry(p)),
    'instance.launch.done': (p) =>
      entryKeys(
        'instances',
        entryFromProcess(String(p.processId ?? ''), 'instance-'),
      ),
    'java.install.done': () => [keys.java.runtimes()],
    // Backup jobs are server-only; content jobs carry only a job id, which
    // another client's jobs mint themselves — the entry is unknowable, so
    // sweep the kind's list (the footprint walk no longer rides under it).
    'backup.done': () => [keys.servers.list()],
    'content.done': () => [keys.servers.list(), keys.instances.list()],
  };

/** The key prefixes a daemon topic outdates — exported for the regression test. */
export function invalidationKeys(
  topic: string,
  payload: Record<string, unknown>,
): QueryKey[] {
  return TOPICS[topic]?.(payload) ?? [];
}

let started = false;

/** Install the feed once, at app bootstrap. */
export function startInvalidation(): void {
  if (started) return;
  started = true;
  onDaemonEvent((event) => {
    for (const key of invalidationKeys(event.topic, event.payload))
      invalidate(key);
  }).catch(() => {
    // Outside the Tauri shell there are no daemon events to hear.
  });
  onConnectionChange((state) => {
    // The daemon may have changed while we were away, so refresh everything —
    // except the footprint walks, which don't drift on their own and are
    // expensive; they refetch on next mount if stale.
    if (state === 'connected')
      void queryClient.invalidateQueries({
        predicate: (query) => query.queryKey[0] !== FOOTPRINT,
      });
  }).catch(() => {
    // Same: no bridge, nothing to watch.
  });
}
