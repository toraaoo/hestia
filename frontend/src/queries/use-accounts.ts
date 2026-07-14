/**
 * Accounts: the signed-in list plus the login/switch/remove actions. The
 * desktop signs in over sisu: `beginLogin()` yields the Microsoft URL to
 * open, `completeLogin(id, code)` redeems the OAuth code captured from the
 * redirect (it stays pending while the user signs in — up to 16 minutes).
 */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import type { LoginMethod } from '../api';
import { accounts } from '../api';
import { sweeper } from './client';
import { keys } from './keys';

export function useAccounts() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.accounts, queryFn: accounts.list });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.accounts);
    return {
      beginLogin: (method: LoginMethod = 'sisu') => accounts.beginLogin(method),
      completeLogin: (id: string, code?: string) =>
        done(accounts.completeLogin(id, code)),
      /** Pick the default account launches use; name or uuid. */
      switchAccount: (account: string) => done(accounts.switchAccount(account)),
      removeAccount: (account: string) => done(accounts.remove(account)),
    };
  }, [queryClient]);
  return { ...query, ...actions };
}
