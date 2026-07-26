import type { GameVersion } from './types/minecraft';

/**
 * Whether moving `from` → `to` is a downgrade, judged by position in the
 * flavor's newest-first catalogue (mirrors `proto::minecraft::downgrade_between`).
 * `null` when either version is not listed.
 */
export function downgradeBetween(
  versions: GameVersion[],
  from: string,
  to: string,
): boolean | null {
  const fromIndex = versions.findIndex((v) => v.id === from);
  const toIndex = versions.findIndex((v) => v.id === to);
  if (fromIndex < 0 || toIndex < 0) return null;
  return toIndex > fromIndex;
}
