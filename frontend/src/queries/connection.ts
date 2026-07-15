/**
 * Daemon connection state as a React subscription. Optimistically
 * `connected` until the shell's watcher reports a transition — the bridge
 * auto-spawns the daemon on the first call, so pessimism would just flash a
 * disconnected banner at startup.
 */
import { useSyncExternalStore } from 'react';
import { type ConnectionState, onConnectionChange } from '../api';

let state: ConnectionState = 'connected';
const listeners = new Set<() => void>();
let watching = false;

function ensureWatcher(): void {
  if (watching) return;
  watching = true;
  onConnectionChange((next) => {
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
