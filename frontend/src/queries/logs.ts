/**
 * Live log following, shared by the server/instance/process log hooks: a
 * fetched tail (the query) plus `process.output` events accumulated on top.
 * The buffer resets whenever a fresh tail lands, so a refetch never
 * duplicates lines it already contains.
 */
import type { UseQueryResult } from '@tanstack/react-query';
import { useMemo, useRef, useState } from 'react';
import type { HestiaError, ProcessLogLine } from '../api';
import { useDaemonEvent } from './events';

export interface LogsOptions {
  /** How many lines of history to fetch. */
  tail?: number;
  /** Accumulate `process.output` events on top of the fetched tail. */
  follow?: boolean;
  /** Cap on accumulated live lines; default 1000. */
  limit?: number;
}

export type LogsResult = UseQueryResult<ProcessLogLine[], HestiaError> & {
  /** The fetched tail plus everything streamed since. */
  lines: ProcessLogLine[];
};

// The daemon coalesces a tail poll's lines into one event, so the wire shape is
// `{ id, lines: ProcessLogLine[] }` — a batch, not a single line.
interface ProcessOutputPayload {
  id: string;
  lines: ProcessLogLine[];
}

export function useFollowedLogs(
  query: UseQueryResult<ProcessLogLine[], HestiaError>,
  matches: ((processId: string) => boolean) | null,
  limit = 1000,
): LogsResult {
  const [live, setLive] = useState<ProcessLogLine[]>([]);
  const matchesRef = useRef(matches);
  matchesRef.current = matches;

  const [seenFetchAt, setSeenFetchAt] = useState(query.dataUpdatedAt);
  if (seenFetchAt !== query.dataUpdatedAt) {
    setSeenFetchAt(query.dataUpdatedAt);
    setLive([]);
  }

  useDaemonEvent<ProcessOutputPayload>('process.output', (payload) => {
    if (!matchesRef.current?.(payload.id) || payload.lines.length === 0) return;
    setLive((lines) => {
      const merged = [...lines, ...payload.lines];
      return merged.length > limit
        ? merged.slice(merged.length - limit)
        : merged;
    });
  });

  const lines = useMemo(
    () => [...(query.data ?? []), ...live],
    [query.data, live],
  );
  return { ...query, lines };
}
