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
import { keys } from './keys';

function processKeys(payload: Record<string, unknown>): QueryKey[] {
  const id = String(payload.id ?? '');
  if (id.startsWith('server-')) return [keys.processes.all, keys.servers.all];
  if (id.startsWith('instance-'))
    return [keys.processes.all, keys.instances.all];
  return [keys.processes.all];
}

const TOPICS: Record<string, (payload: Record<string, unknown>) => QueryKey[]> =
  {
    'process.started': processKeys,
    'process.exit': processKeys,
    'server.create.done': () => [keys.servers.all],
    // A failed create removes the half-provisioned record.
    'server.create.error': () => [keys.servers.all],
    'server.update.done': () => [keys.servers.all],
    'instance.launch.done': () => [keys.instances.all],
    'java.install.done': () => [keys.java.all],
    // Backup jobs are server-only; content jobs carry only a job id, which
    // another client's jobs mint themselves — the entry is unknowable, so
    // sweep both kinds.
    'backup.done': () => [keys.servers.all],
    'content.done': () => [keys.servers.all, keys.instances.all],
  };

let started = false;

/** Install the feed once, at app bootstrap. */
export function startInvalidation(): void {
  if (started) return;
  started = true;
  onDaemonEvent((event) => {
    const mapped = TOPICS[event.topic];
    if (!mapped) return;
    for (const key of mapped(event.payload)) invalidate(key);
  }).catch(() => {
    // Outside the Tauri shell there are no daemon events to hear.
  });
  onConnectionChange((state) => {
    if (state === 'connected') void queryClient.invalidateQueries();
  }).catch(() => {
    // Same: no bridge, nothing to watch.
  });
}
