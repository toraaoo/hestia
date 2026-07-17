/**
 * The `account.*` channels. The desktop signs in over the sisu flow:
 * `beginLogin("sisu")` returns the Microsoft sign-in `url` for the shell to
 * open, and `completeLogin(id, code)` redeems the OAuth code captured from
 * the redirect. The device-code flow shares the same two calls with
 * `user_code`/`verification_uri` instead — `completeLogin` then polls until
 * the user approves, hence the long timeout on both variants.
 */
import { call, invokeCommand } from './core/ipc';
import type {
  Account,
  AccountList,
  LoginBegin,
  LoginMethod,
} from './types/accounts';

/**
 * Sign in a Microsoft account over the sisu flow. The shell command opens the
 * Microsoft sign-in page in a native webview, catches the OAuth redirect, and
 * drives `account.login.begin` → `account.login.complete` around it — reading
 * the cross-origin redirect URL is only possible shell-side. Resolves to the
 * stored account, or `null` when the user closes the window before finishing.
 */
export function loginSisu(): Promise<Account | null> {
  return invokeCommand<Account | null>('account_login_sisu');
}

export function beginLogin(method: LoginMethod = 'sisu'): Promise<LoginBegin> {
  return call('account.login.begin', { method }, { timeoutMs: 60_000 });
}

export async function completeLogin(id: string, code = ''): Promise<Account> {
  const result = await call<{ account: Account }>(
    'account.login.complete',
    { id, code },
    { timeoutMs: 16 * 60_000 },
  );
  return result.account;
}

export function list(): Promise<AccountList> {
  return call('account.list');
}

/** Pick the default account launches use; `account` is a name or uuid. */
export async function switchAccount(account: string): Promise<Account> {
  const result = await call<{ account: Account }>('account.switch', {
    account,
  });
  return result.account;
}

export async function remove(account: string): Promise<void> {
  await call('account.remove', { account });
}
