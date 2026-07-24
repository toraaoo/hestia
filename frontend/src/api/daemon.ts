/** The `daemon.*` channels. */
import { call, invokeCommand } from './core/ipc';
import type { DaemonStatus } from './types/daemon';

/** The old daemon must release the endpoint before a restart respawns. */
const RESTART_GRACE_MS = 600;

export function status(): Promise<DaemonStatus> {
  return call('daemon.status');
}

/** Without `stopProcesses`, supervised workloads keep running. */
export async function stop(stopProcesses = false): Promise<boolean> {
  const result = await call<{ stopping: boolean }>('daemon.stop', {
    stopProcesses,
  });
  return result.stopping;
}

/** Start the daemon via the shell's `start_daemon` command; `ipc_call` never spawns. */
export async function start(): Promise<DaemonStatus> {
  await invokeCommand('start_daemon');
  return status();
}

/**
 * Stop then start again — picks up a freshly built `hestiad`; supervised
 * processes are re-adopted. The grace lets the old daemon release the endpoint.
 */
export async function restart(): Promise<DaemonStatus> {
  await stop(false);
  await delay(RESTART_GRACE_MS);
  return start();
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
