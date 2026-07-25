/**
 * Daemon connection state as a React subscription. Optimistically `connected`
 * until the shell reports a transition (the bridge emits `disconnected` when it
 * cannot reach the daemon); the daemon is commonly up via autostart, so
 * pessimism would just flash a disconnected banner at startup.
 */
import { useSyncExternalStore } from 'react';

import { logger } from '@/lib/log';
import { type ConnectionState, onConnectionChange } from '../api';

const log = logger('connection');

let state: ConnectionState = 'connected';
const listeners = new Set<() => void>();
let watching = false;

function ensureWatcher(): void {
  if (watching) return;
  watching = true;
  onConnectionChange((next) => {
    if (next !== state)
      log.warn({ from: state, to: next }, 'daemon connection');
    state = next;
    for (const listener of listeners) listener();
  }).catch(() => {
    // Outside the Tauri shell (plain `vite dev`) there is no bridge to watch.
  });
}

function subscribe(listener: () => void): () => void {
  ensureWatcher();
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useConnection(): ConnectionState {
  return useSyncExternalStore(subscribe, () => state);
}
