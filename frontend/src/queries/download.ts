/** `download.start` — a job mutation over the engine's verifying downloader. */
import type { DownloadProgress, DownloadSpec } from '../api';
import * as api from '../api/download';
import { jobMutation, useJobMutation } from './jobs';

export const downloadMutations = {
  start: () =>
    jobMutation<
      { id: string; path: string },
      Omit<DownloadSpec, 'id'>,
      DownloadProgress
    >({
      mutationKey: ['downloads', 'start'],
      meta: (spec) => ({ kind: 'download', label: `download ${spec.url}` }),
      run: (spec, onProgress) => api.start(spec, onProgress),
    }),
};

export function useStartDownload() {
  return useJobMutation(downloadMutations.start());
}
