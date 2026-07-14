/** Mirrors `crates/proto/src/accounts.rs`. */

export type LoginMethod = 'device_code' | 'sisu';

export interface Account {
  uuid: string;
  name: string;
}

/**
 * What `account.login.begin` hands back for the user to act on: sisu fills
 * `url` (the Microsoft sign-in page whose redirect carries the OAuth code);
 * device_code fills `user_code` + `verification_uri`.
 */
export interface LoginBegin {
  id: string;
  method: LoginMethod;
  url?: string;
  user_code?: string;
  verification_uri?: string;
}

export interface AccountList {
  accounts: Account[];
  /** The account launches use when none is named. */
  default_uuid?: string;
}
