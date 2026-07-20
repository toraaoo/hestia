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
import { type HestiaError, UNAUTHORIZED } from '../api';

// The instance surface is gated on a signed-in account (the router answers
// `unauthorized` before an account exists). Every gated feature is already
// hidden behind the sign-in UI, so these are expected on first launch — never
// a toast.
const silent = (error: HestiaError) => error.code === UNAUTHORIZED;

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
      // A query may opt out of the toast (e.g. a ping a stopped server can't
      // answer) via meta.silent.
      if (query.meta?.silent || silent(error)) return;
      toast.error(error.message, { id: query.queryHash });
    },
  }),
  mutationCache: new MutationCache({
    onError: (error) => {
      if (silent(error)) return;
      toast.error(error.message);
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
