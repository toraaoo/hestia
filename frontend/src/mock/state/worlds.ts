/**
 * What an instance's `data/` holds that the launcher reads back out: the save
 * worlds described from `level.dat`, and the multiplayer list (`servers.dat`).
 * Both are editable from the UI, so both are mutable here.
 */
import type { ServerEntry, WorldInfo } from '@/api/types';

import { ago } from '../support';

const DEFAULT_WORLDS: WorldInfo[] = [
  {
    folder: 'New World',
    name: 'New World',
    read: true,
    version: '1.21.1',
    gameMode: 'survival',
    difficulty: 'normal',
    hardcore: false,
    cheats: false,
    lastPlayedUnix: ago(3_600),
    sizeBytes: 128 * 1024 * 1024,
    icon: '',
  },
  {
    folder: 'creative-flats',
    name: 'Creative Flats',
    read: true,
    version: '1.21.1',
    gameMode: 'creative',
    difficulty: 'peaceful',
    hardcore: false,
    cheats: true,
    lastPlayedUnix: ago(86_400 * 9),
    sizeBytes: 12 * 1024 * 1024,
    icon: '',
  },
  {
    folder: 'hardcore-run',
    name: 'Hardcore Run',
    read: true,
    version: '1.21.1',
    gameMode: 'survival',
    difficulty: 'hard',
    hardcore: true,
    cheats: false,
    lastPlayedUnix: ago(86_400 * 2),
    sizeBytes: 64 * 1024 * 1024,
    icon: '',
  },
];

const DEFAULT_SERVERS: ServerEntry[] = [
  {
    name: 'Mock SMP',
    address: 'smp.example.net',
    icon: '',
    acceptTextures: true,
    hidden: false,
  },
  {
    name: 'Creative Realm',
    address: 'creative.example.net:25566',
    icon: '',
    acceptTextures: false,
    hidden: false,
  },
];

const worlds = new Map<string, WorldInfo[]>([
  ['fabric-playground', DEFAULT_WORLDS.map((world) => ({ ...world }))],
  ['vanilla-survival', [{ ...DEFAULT_WORLDS[0] }]],
]);

const multiplayer = new Map<string, ServerEntry[]>([
  ['fabric-playground', DEFAULT_SERVERS.map((entry) => ({ ...entry }))],
]);

export const worldsOf = (id: string): WorldInfo[] => worlds.get(id) ?? [];

export function serversOf(id: string): ServerEntry[] {
  const existing = multiplayer.get(id);
  if (existing) return existing;
  const fresh: ServerEntry[] = [];
  multiplayer.set(id, fresh);
  return fresh;
}

const at = (entries: ServerEntry[], ref: string): number =>
  entries.findIndex((entry) => entry.name === ref || entry.address === ref);

/** Rewrite the entry `ref` names, or append when it names none. */
export function editServer(
  id: string,
  ref: string,
  entry: ServerEntry,
): ServerEntry[] {
  const entries = serversOf(id);
  const index = ref ? at(entries, ref) : -1;
  if (index >= 0) entries[index] = entry;
  else entries.push(entry);
  return entries;
}

export function removeServer(id: string, ref: string): ServerEntry[] {
  const entries = serversOf(id);
  const index = at(entries, ref);
  if (index >= 0) entries.splice(index, 1);
  return entries;
}
