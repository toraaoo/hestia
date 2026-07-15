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
  /** Key prefixes swept when the mutation settles, success or error. */
  invalidates?: (variables: TVariables) => QueryKey[];
}

export function mutation<TData = void, TVariables = void>(
  spec: MutationSpec<TData, TVariables>,
): UseMutationOptions<TData, HestiaError, TVariables> {
  return {
    mutationKey: spec.mutationKey,
    mutationFn: spec.mutationFn,
    onSettled: (_data, _error, variables) => {
      for (const key of spec.invalidates?.(variables) ?? []) invalidate(key);
    },
  };
}
