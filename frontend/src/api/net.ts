/** The `net.*` channels. */
import { call } from './core/ipc';
import type { NetworkStatus } from './types/net';

/** Whether the daemon can currently reach upstream. */
export function status(): Promise<NetworkStatus> {
  return call('net.status');
}
