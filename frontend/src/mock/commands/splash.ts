/**
 * `crates/desktop/src/commands/splash.rs` — the webview reporting that it has
 * painted. A browser tab has no splash window to close, so this only has to
 * answer rather than fall through to the unknown-command warning.
 */
import type { Handlers } from '../support';

export const commands: Handlers = {
  ready: () => null,
};
