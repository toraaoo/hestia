/** The `daemon.*` channels. */
import { call } from './core/ipc';
import type { DaemonStatus } from './types/daemon';

export function status(): Promise<DaemonStatus> {
  return call('daemon.status');
}

/** Without `stopProcesses`, supervised workloads keep running. */
export async function stop(stopProcesses = false): Promise<boolean> {
  const result = await call<{ stopping: boolean }>('daemon.stop', {
    stop_processes: stopProcesses,
  });
  return result.stopping;
}
