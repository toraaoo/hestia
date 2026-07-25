/**
 * The one `QueryClient` the app runs on — a module singleton so the mutation
 * factories and the event-driven invalidation feed can reach it without a
 * React context.
 */
import {
  MutationCache,
  QueryCache,
  QueryClient,
  type QueryKey,
} from '@tanstack/react-query';
import { toast } from 'sonner';

import { logger } from '@/lib/log';
import {
  CONNECTION_LOST,
  errorMessage,
  type HestiaError,
  TIMEOUT,
  TRANSPORT,
  UNAUTHORIZED,
} from '../api';

const log = logger('query');

// Never a toast: `unauthorized` is expected behind the sign-in gate, and
// connectivity failures are the status bar's concern, not a per-call error.
const TRANSPORT_CODES = new Set([TRANSPORT, CONNECTION_LOST, TIMEOUT]);
const silent = (error: HestiaError) =>
  error.code === UNAUTHORIZED || TRANSPORT_CODES.has(error.code);

declare module '@tanstack/react-query' {
  interface Register {
    defaultError: HestiaError;
  }
}

export const queryClient = new QueryClient({
  // Failures toast instead of rendering into pages; the query hash id keeps
  // a retriggering refetch replacing its own toast rather than stacking.
  queryCache: new QueryCache({
    onError: (error, query) => {
      log.warn(
        { key: query.queryHash, code: error.code },
        `query failed: ${error.message}`,
      );
      // A query may opt out of the toast (e.g. a ping a stopped server can't
      // answer) via meta.silent.
      if (query.meta?.silent || silent(error)) return;
      toast.error(errorMessage(error), { id: query.queryHash });
    },
  }),
  mutationCache: new MutationCache({
    onError: (error, _vars, _ctx, mutation) => {
      log.warn(
        { key: mutation.options.mutationKey?.join('.'), code: error.code },
        `mutation failed: ${error.message}`,
      );
      if (silent(error)) return;
      toast.error(errorMessage(error));
    },
  }),
  defaultOptions: {
    queries: {
      // The daemon is a local socket, not HTTP: the webview's online/offline
      // signal is meaningless here, and failures are not transient network
      // blips worth retrying.
      networkMode: 'always',
      retry: false,
      // Daemon events invalidate what changes, so polling-style refetches
      // only need to catch what the topic map misses.
      staleTime: 30_000,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
    },
    mutations: {
      networkMode: 'always',
      retry: false,
    },
  },
});

/** Sweep every query under the key prefix. */
export function invalidate(key: QueryKey): void {
  void queryClient.invalidateQueries({ queryKey: key });
}
