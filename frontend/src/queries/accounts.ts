/**
 * `account.*` — queries/mutations plus their 1:1 hooks. Sign-in is the
 * two-step flow: `useBeginLogin` yields what the user must act on (the sisu
 * URL, or a device code), `useCompleteLogin` blocks until the account is
 * stored — hence its long-lived pending state.
 */
import type { QueryClient } from '@tanstack/react-query';
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
  /**
   * The desktop sign-in: one shell command drives the whole sisu flow behind a
   * native Microsoft window. Resolves to the new account, or `null` on cancel.
   */
  loginSisu: () =>
    mutation<Account | null, void>({
      mutationKey: [...keys.accounts.all, 'login', 'sisu'],
      mutationFn: () => api.loginSisu(),
      invalidates: () => [keys.accounts.all],
    }),
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
  const query = useQuery(accountQueries.list());
  const login = useMutation(accountMutations.loginSisu());
  const beginLogin = useMutation(accountMutations.beginLogin());
  const completeLogin = useMutation(accountMutations.completeLogin());
  const switchAccount = useMutation(accountMutations.switch());
  const remove = useMutation(accountMutations.remove());

  const accounts = query.data?.accounts ?? [];
  const active =
    accounts.find((a) => a.uuid === query.data?.default_uuid) ?? accounts[0];

  return {
    accounts,
    active,
    signedIn: accounts.length > 0,
    isPending: query.isPending,
    ready: !query.isPending,
    login,
    beginLogin,
    completeLogin,
    switch: switchAccount,
    remove,
  };
}

export async function ensureSignedIn(
  queryClient: QueryClient,
): Promise<boolean> {
  const list = await queryClient.ensureQueryData(accountQueries.list());
  return list.accounts.length > 0;
}
