/** The `process.*` channels — thin over the daemon's supervisor. */
import { call } from './core/ipc';
import type { ProcessInfo, ProcessLogLine, ProcessSpec } from './types/process';

export function start(spec: ProcessSpec): Promise<{ id: string; pid: number }> {
  return call('process.start', spec);
}

export async function stop(id: string): Promise<void> {
  await call('process.stop', { id });
}

export async function list(): Promise<ProcessInfo[]> {
  const result = await call<{ processes: ProcessInfo[] }>('process.list');
  return result.processes;
}

export function status(id: string): Promise<ProcessInfo> {
  return call('process.status', { id });
}

export async function logs(
  id: string,
  tail?: number,
): Promise<ProcessLogLine[]> {
  const result = await call<{ lines: ProcessLogLine[] }>('process.logs', {
    id,
    tail,
  });
  return result.lines;
}
