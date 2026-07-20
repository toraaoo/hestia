/**
 * Custom entry icons: desktop-local, copied into `<data_home>/icons/` by the
 * shell (`icons_*` commands) and served over the asset protocol. Keyed by the
 * entry's stable id; the disk is the registry.
 */
import { convertFileSrc } from '@tauri-apps/api/core';
import { invokeCommand } from './core/ipc';

/** One stored icon; `mtime` doubles as the cache-busting version. */
export interface IconEntry {
  path: string;
  mtime: number;
}

export function list(): Promise<Record<string, IconEntry>> {
  return invokeCommand('icons_list');
}

/** Copy a picked image into the data home as `entryId`'s icon. */
export function set(entryId: string, sourcePath: string): Promise<IconEntry> {
  return invokeCommand('icon_set', { entryId, sourcePath });
}

export function remove(entryId: string): Promise<void> {
  return invokeCommand('icon_remove', { entryId });
}

/** The webview-loadable URL for a stored icon. */
export function iconUrl(entry: IconEntry): string {
  return `${convertFileSrc(entry.path)}?v=${entry.mtime}`;
}
