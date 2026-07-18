/**
 * The plain-mutation factory: typed `UseMutationOptions` over an API call,
 * with invalidation declared as data. Every non-job mutation in the domain
 * files is one `mutation({ ... })`; job-backed operations use `jobMutation`
 * from `./jobs` instead.
 */
import type { QueryKey, UseMutationOptions } from '@tanstack/react-query';
import type { HestiaError } from '../api';
import { invalidate } from './client';

export interface MutationSpec<TData, TVariables> {
  mutationKey: QueryKey;
  mutationFn: (variables: TVariables) => Promise<TData>;
  /**
   * Apply the expected outcome to the cache before the daemon answers; the
   * returned function undoes it, run on error so the UI reconciles back.
   * The settle-time invalidation then refetches the daemon's truth either way.
   */
  optimistic?: (variables: TVariables) => (() => void) | undefined;
  /** Key prefixes swept when the mutation settles, success or error. */
  invalidates?: (variables: TVariables) => QueryKey[];
}

interface OptimisticContext {
  undo?: () => void;
}

export function mutation<TData = void, TVariables = void>(
  spec: MutationSpec<TData, TVariables>,
): UseMutationOptions<TData, HestiaError, TVariables, OptimisticContext> {
  return {
    mutationKey: spec.mutationKey,
    mutationFn: spec.mutationFn,
    onMutate: spec.optimistic
      ? (variables) => ({ undo: spec.optimistic?.(variables) })
      : undefined,
    onError: (_error, _variables, context) => {
      context?.undo?.();
    },
    onSettled: (_data, _error, variables) => {
      for (const key of spec.invalidates?.(variables) ?? []) invalidate(key);
    },
  };
}
