/**
 * The one `QueryClient` the app runs on — a module singleton so the mutation
 * factories and the event-driven invalidation feed can reach it without a
 * React context.
 */
import {
  MutationCache,
  QueryClient,
  type QueryKey,
} from '@tanstack/react-query';
import { toast } from 'sonner';
import type { HestiaError } from '../api';

declare module '@tanstack/react-query' {
  interface Register {
    defaultError: HestiaError;
  }
}

export const queryClient = new QueryClient({
  // Every mutation failure surfaces as a toast: optimistic surfaces have
  // already rolled back by the time this fires, so the toast is the one
  // signal the user gets that the change did not stick.
  mutationCache: new MutationCache({
    onError: (error) => {
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
