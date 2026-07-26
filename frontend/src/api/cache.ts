/** The `cache.*` channels. */
import { call } from './core/ipc';
import type { CacheEntry, CacheInfoResult, CacheUsage } from './types/cache';

export function info(): Promise<CacheInfoResult> {
  return call('cache.info');
}

export async function list(): Promise<CacheEntry[]> {
  const result = await call<{ entries: CacheEntry[] }>('cache.list');
  return result.entries;
}

/** Returns what was reclaimed. */
export function clear(): Promise<CacheUsage> {
  return call('cache.clear');
}
