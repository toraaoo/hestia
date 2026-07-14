/**
 * The `download.start` channel: stream a URL to a daemon-local path through
 * the engine's verifying downloader. Resolves with the written path.
 */

import { call } from './core/ipc';
import { jobId, runJob } from './core/jobs';
import type { DownloadProgress, DownloadSpec } from './types/download';

export function start(
  spec: Omit<DownloadSpec, 'id'>,
  onProgress?: (progress: DownloadProgress) => void,
): Promise<{ id: string; path: string }> {
  const id = jobId('download');
  return runJob<{ id: string; path: string }, DownloadProgress>({
    id,
    topics: {
      progress: 'download.progress',
      done: 'download.done',
      error: 'download.error',
    },
    onProgress,
    start: () => call('download.start', { ...spec, id }),
  });
}
