/** Mirrors `crates/proto/src/cache.rs`. */
import type { Checksum } from './download';

/** `Checksum` is flattened into the entry on the wire. */
export type CacheEntry = Checksum & {
  size: number;
};

export interface CacheUsage {
  entries: number;
  bytes: number;
}

/** `CacheUsage` is flattened beside `path` on the wire. */
export type CacheInfo = CacheUsage & {
  path: string;
};
