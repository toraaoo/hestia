/**
 * Event-driven freshness: the bridge forwards every daemon event, so lists
 * stay current without polling. Only terminal topics invalidate — progress
 * topics fire constantly and are consumed by job callbacks instead. The map
 * is deliberately coarse (a backup done event cannot say which entry kind it
 * belongs to); refetching against a local daemon is cheap.
 */
import type { QueryClient, QueryKey } from '@tanstack/react-query';
import { onConnectionChange, onDaemonEvent } from '../api';
import { keys } from './keys';

const entryKeys: QueryKey[] = [keys.servers.all, keys.instances.all];

const topicKeys: Record<string, QueryKey[]> = {
  'process.started': [...entryKeys, keys.processes],
  'process.exit': [...entryKeys, keys.processes],
  'server.create.done': [keys.servers.all],
  'server.create.error': [keys.servers.all],
  'server.update.done': [keys.servers.all],
  'instance.launch.done': [keys.instances.all],
  'backup.done': entryKeys,
  'content.done': entryKeys,
  'java.install.done': [keys.java.all, keys.cache],
};

/**
 * Install the daemon-event → invalidation map on a query client. Call once,
 * next to the provider. Returns an uninstaller.
 */
export async function installDaemonInvalidation(
  queryClient: QueryClient,
): Promise<() => void> {
  const offEvents = await onDaemonEvent((event) => {
    const targets = topicKeys[event.topic];
    if (!targets) return;
    for (const queryKey of targets) {
      void queryClient.invalidateQueries({ queryKey });
    }
  });
  const offConnection = await onConnectionChange((state) => {
    // A reconnect means an unknown stretch of missed events — refetch all.
    if (state === 'connected') void queryClient.invalidateQueries();
  });
  return () => {
    offEvents();
    offConnection();
  };
}
