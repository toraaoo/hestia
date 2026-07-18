/**
 * Per-server console history: the command echoes and one-shot RCON replies a
 * user sends from the console tab. Held in a module store keyed by server id so
 * it outlives the tab (base-ui unmounts a hidden panel) and page navigation —
 * the daemon's log file persists server output, but a command's echo and its
 * RCON reply live only here.
 */
import { useSyncExternalStore } from 'react';

export interface ConsoleEntry {
  kind: 'echo' | 'reply' | 'error';
  text: string;
}

const MAX_ENTRIES = 500;
const EMPTY: ConsoleEntry[] = [];

const histories = new Map<string, ConsoleEntry[]>();
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function pushConsoleEntry(id: string, entry: ConsoleEntry): void {
  const prev = histories.get(id) ?? EMPTY;
  const next = [...prev, entry];
  histories.set(
    id,
    next.length > MAX_ENTRIES ? next.slice(next.length - MAX_ENTRIES) : next,
  );
  for (const listener of listeners) listener();
}

export function useConsoleHistory(id: string): ConsoleEntry[] {
  return useSyncExternalStore(subscribe, () => histories.get(id) ?? EMPTY);
}
