/**
 * The `download.start` channel: stream a URL to a daemon-local path through
 * the engine's verifying downloader. Resolves with the written path.
 */

import { call } from './core/ipc';
import { type JobRun, runJob } from './core/jobs';
import type { DownloadProgress, DownloadSpec } from './types/download';

export function start(
  spec: Omit<DownloadSpec, 'id'>,
  job: JobRun<DownloadProgress>,
): Promise<{ id: string; path: string }> {
  return runJob<{ id: string; path: string }, DownloadProgress>({
    ...job,
    topics: {
      progress: 'download.progress',
      done: 'download.done',
      error: 'download.error',
    },
    start: () => call('download.start', { ...spec, id: job.id }),
  });
}
