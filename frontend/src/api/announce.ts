/**
 * The `announce.*` channels — the news and notices fetched from the published
 * feed. Reads answer from the daemon's cache; only `refresh` touches the
 * network, so it takes the longer timeout the Rust facade uses.
 */
import { call } from './core/ipc';
import type { AnnounceListResult } from './types/announce';

const REFRESH_TIMEOUT_MS = 30_000;

/** Everything that applies to this build, newest first, dismissed flagged. */
export function list(): Promise<AnnounceListResult> {
  return call('announce.list');
}

/** Mark announcements read. */
export function dismiss(ids: string[]): Promise<AnnounceListResult> {
  return call('announce.dismiss', { ids });
}

/**
 * Fetch the feed now rather than waiting for the daemon's poll. Answers from
 * cache if the fetch fails, so this does not reject on a dead network.
 */
export function refresh(): Promise<AnnounceListResult> {
  return call('announce.refresh', {}, { timeoutMs: REFRESH_TIMEOUT_MS });
}
