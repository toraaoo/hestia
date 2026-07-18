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

// The daemon's `ProcessOutputEvent` flattens its `ProcessLogLine`, so the wire
// shape is `{ id, stream, line }` — the log line's fields sit beside `id`, not
// nested under `line`.
interface ProcessOutputPayload extends ProcessLogLine {
  id: string;
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
    if (!matchesRef.current?.(payload.id)) return;
    const next: ProcessLogLine = {
      stream: payload.stream,
      line: payload.line,
    };
    setLive((lines) =>
      lines.length >= limit
        ? [...lines.slice(lines.length - limit + 1), next]
        : [...lines, next],
    );
  });

  const lines = useMemo(
    () => [...(query.data ?? []), ...live],
    [query.data, live],
  );
  return { ...query, lines };
}
