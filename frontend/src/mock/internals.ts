/**
 * Installing `window.__TAURI_INTERNALS__` — the object every
 * `@tauri-apps/api` call goes through. Three pieces are needed before the app
 * touches any of them:
 *
 * - the IPC hook, routed to ../router (`shouldMockEvents` makes the event
 *   plugin real, so `listen`/`emit` work and the daemon events in ./bus land);
 * - the window metadata `getCurrentWindow()`/`getCurrentWebview()` read;
 * - `convertFileSrc`, which is a bare property access and throws without one.
 */
import {
  mockConvertFileSrc,
  mockIPC,
  mockWindows,
} from '@tauri-apps/api/mocks';

import { dispatch } from './router';

/** What the asset protocol would look like on the host we appear to run on. */
function platform(): 'linux' | 'macos' | 'windows' {
  const agent = navigator.userAgent;
  if (agent.includes('Windows')) return 'windows';
  if (agent.includes('Mac OS')) return 'macos';
  return 'linux';
}

export function installInternals(): void {
  mockWindows('main');
  mockConvertFileSrc(platform());
  mockIPC(
    (cmd, args) => dispatch(cmd, (args ?? {}) as Record<string, unknown>),
    { shouldMockEvents: true },
  );
}
