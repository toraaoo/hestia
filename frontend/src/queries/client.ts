import { QueryClient, type QueryKey } from '@tanstack/react-query';
import { HestiaError } from '#/api';

const NO_RETRY_CODES = new Set([
  'not_found',
  'bad_request',
  'unknown_channel',
  'handler_error',
]);

/**
 * Query defaults tuned for a local daemon: refetches are cheap, but a typed
 * daemon error is deterministic — retrying it cannot help. Freshness comes
 * primarily from event-driven invalidation (`invalidation.ts`), not focus
 * refetching, which a desktop webview triggers constantly.
 */
/**
 * The settle-then-invalidate wrapper every action maker uses: run the call,
 * then sweep the domain's key prefix whether it succeeded or not (a failure
 * may still have changed daemon state). Rejections pass through untouched.
 */
export function sweeper(queryClient: QueryClient, queryKey: QueryKey) {
  return async <T>(promise: Promise<T>): Promise<T> => {
    try {
      return await promise;
    } finally {
      void queryClient.invalidateQueries({ queryKey });
    }
  };
}

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        refetchOnWindowFocus: false,
        staleTime: 5_000,
        retry(failureCount, error) {
          if (error instanceof HestiaError && NO_RETRY_CODES.has(error.code)) {
            return false;
          }
          return failureCount < 2;
        },
      },
    },
  });
}
