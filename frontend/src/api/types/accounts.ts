/** Mirrors `crates/proto/src/accounts.rs`. */

export type LoginMethod = 'device_code' | 'sisu';

export interface Account {
  uuid: string;
  name: string;
  /** Stored, but its refresh token was rejected: cannot launch until re-login. */
  needsReauth: boolean;
}

/**
 * What `account.login.begin` hands back for the user to act on: sisu fills
 * `url` (the Microsoft sign-in page whose redirect carries the OAuth code);
 * device_code fills `userCode` + `verificationUri`.
 */
export interface LoginBegin {
  id: string;
  method: LoginMethod;
  url?: string;
  userCode?: string;
  verificationUri?: string;
}

export interface AccountList {
  accounts: Account[];
  /** The account launches use when none is named. */
  defaultUuid?: string;
}
