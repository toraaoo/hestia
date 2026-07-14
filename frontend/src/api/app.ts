/** The `app.*` / `health.*` channels. */
import { call } from './core/ipc';
import type { AppInfo, PingResult } from './types/app';

export function info(): Promise<AppInfo> {
  return call('app.info');
}

export function ping(): Promise<PingResult> {
  return call('health.ping');
}
