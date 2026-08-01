/** The `update.*` channels. */

import { call } from './core/ipc';
import { type JobRun, runJob } from './core/jobs';
import type { DownloadProgress } from './types/download';
import type {
  UpdateApplyResult,
  UpdateCheckResult,
  UpdateDoneEvent,
} from './types/update';

export function check(): Promise<UpdateCheckResult> {
  return call<UpdateCheckResult>('update.check');
}

export function download(
  job: JobRun<DownloadProgress>,
): Promise<UpdateDoneEvent> {
  return runJob<UpdateDoneEvent, DownloadProgress>({
    ...job,
    topics: {
      progress: 'update.progress',
      done: 'update.done',
      error: 'update.error',
    },
    start: () => call('update.download', { id: job.id }),
  });
}

/** Waits on an interactive elevation prompt on Linux, so the timeout is long. */
export function apply(path: string): Promise<UpdateApplyResult> {
  return call<UpdateApplyResult>(
    'update.apply',
    { path },
    { timeoutMs: 300_000 },
  );
}
