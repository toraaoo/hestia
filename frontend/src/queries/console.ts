/**
 * Per-server console history: the command echoes and one-shot RCON replies a
 * user sends from the console tab. Held in a module store keyed by server id so
 * it outlives the tab (base-ui unmounts a hidden panel) and page navigation —
 * the daemon's log file persists server output, but a command's echo and its
 * RCON reply live only here.
 */
import { useCallback, useSyncExternalStore } from 'react';

export interface ConsoleEntry {
  kind: 'echo' | 'reply' | 'error';
  text: string;
}

const MAX_ENTRIES = 500;
const EMPTY: ConsoleEntry[] = [];

const histories = new Map<string, ConsoleEntry[]>();
// Listeners are keyed by server id so one server's push wakes only its own
// console, never every open console.
const listeners = new Map<string, Set<() => void>>();

function subscribe(id: string, listener: () => void): () => void {
  let set = listeners.get(id);
  if (!set) {
    set = new Set();
    listeners.set(id, set);
  }
  set.add(listener);
  return () => {
    set.delete(listener);
    if (set.size === 0) listeners.delete(id);
  };
}

export function pushConsoleEntry(id: string, entry: ConsoleEntry): void {
  const prev = histories.get(id) ?? EMPTY;
  const next = [...prev, entry];
  histories.set(
    id,
    next.length > MAX_ENTRIES ? next.slice(next.length - MAX_ENTRIES) : next,
  );
  for (const listener of listeners.get(id) ?? []) listener();
}

export function useConsoleHistory(id: string): ConsoleEntry[] {
  const subscribeId = useCallback(
    (listener: () => void) => subscribe(id, listener),
    [id],
  );
  return useSyncExternalStore(subscribeId, () => histories.get(id) ?? EMPTY);
}
