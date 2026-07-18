/** The `daemon.*` channels. */
import { call } from './core/ipc';
import type { DaemonStatus } from './types/daemon';

/** The old daemon must release the endpoint before the bridge respawns. */
const RESTART_GRACE_MS = 600;

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

/**
 * Start the daemon if it is not running. The shell bridge auto-spawns
 * `hestiad` on any explicit call, so a status read doubles as the trigger.
 */
export function start(): Promise<DaemonStatus> {
  return status();
}

/**
 * Stop then respawn — picks up a freshly built `hestiad`. Supervised
 * processes keep running and are re-adopted.
 */
export async function restart(): Promise<DaemonStatus> {
  await stop(false);
  await delay(RESTART_GRACE_MS);
  return retryStatus();
}

async function retryStatus(attempts = 5): Promise<DaemonStatus> {
  for (let i = 0; ; i++) {
    try {
      return await status();
    } catch (error) {
      if (i >= attempts) throw error;
      await delay(400);
    }
  }
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
