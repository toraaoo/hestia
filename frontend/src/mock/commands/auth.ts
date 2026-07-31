/**
 * `account_login_sisu` — the shell command that opens Microsoft sign-in in a
 * native webview and reads the OAuth redirect back out. There is no webview in
 * a browser, so the fixture answers with the account the flow would have
 * stored (`null` would mean the user closed the window).
 */
import { player } from '../channels/account';
import type { Handlers } from '../support';

export const commands: Handlers = {
  account_login_sisu: () => player,
};
