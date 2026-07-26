/** The `app.*` / `health.*` channels. */
import { call } from './core/ipc';
import type { AppInfoResult } from './types/app';
import type { PingResult } from './types/health';

export function info(): Promise<AppInfoResult> {
  return call('app.info');
}

export function ping(): Promise<PingResult> {
  return call('health.ping');
}
