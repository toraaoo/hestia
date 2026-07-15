/**
 * `account.*` — queries/mutations plus their 1:1 hooks. Sign-in is the
 * two-step flow: `useBeginLogin` yields what the user must act on (the sisu
 * URL, or a device code), `useCompleteLogin` blocks until the account is
 * stored — hence its long-lived pending state.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { Account, LoginBegin, LoginMethod } from '../api';
import * as api from '../api/accounts';
import { mutation } from './core';
import { keys } from './keys';

export const accountQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.accounts.list(),
      queryFn: () => api.list(),
    }),
};

export const accountMutations = {
  beginLogin: () =>
    mutation<LoginBegin, LoginMethod>({
      mutationKey: [...keys.accounts.all, 'login', 'begin'],
      mutationFn: (method) => api.beginLogin(method),
    }),
  completeLogin: () =>
    mutation<Account, { id: string; code?: string }>({
      mutationKey: [...keys.accounts.all, 'login', 'complete'],
      mutationFn: ({ id, code }) => api.completeLogin(id, code),
      invalidates: () => [keys.accounts.all],
    }),
  /** Pick the default account launches use; `account` is a name or uuid. */
  switch: () =>
    mutation<Account, string>({
      mutationKey: [...keys.accounts.all, 'switch'],
      mutationFn: (account) => api.switchAccount(account),
      invalidates: () => [keys.accounts.all],
    }),
  remove: () =>
    mutation<void, string>({
      mutationKey: [...keys.accounts.all, 'remove'],
      mutationFn: (account) => api.remove(account),
      invalidates: () => [keys.accounts.all],
    }),
};

export function useAccounts() {
  return useQuery(accountQueries.list());
}

export function useBeginLogin() {
  return useMutation(accountMutations.beginLogin());
}

export function useCompleteLogin() {
  return useMutation(accountMutations.completeLogin());
}

export function useSwitchAccount() {
  return useMutation(accountMutations.switch());
}

export function useRemoveAccount() {
  return useMutation(accountMutations.remove());
}
