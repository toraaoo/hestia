import { beforeEach, describe, expect, it } from 'vitest';
import { HestiaError } from '../api';
import {
  clearSettledJobs,
  dismissJob,
  getJobs,
  type JobMeta,
  startJob,
} from './jobs';

const meta: JobMeta = {
  kind: 'server.create',
  label: 'create smp',
  entry: { kind: 'server', id: 'smp-3f9a2c7d' },
};

function find(id: string) {
  return getJobs().find((job) => job.id === id);
}

describe('job store', () => {
  beforeEach(() => {
    clearSettledJobs();
  });

  it('tracks progress while running and settles done', async () => {
    let report: (progress: { phase: string }) => void = () => {};
    let finish: (value: string) => void = () => {};
    const handle = startJob<string, { phase: string }>(meta, (onProgress) => {
      report = onProgress;
      return new Promise((resolve) => {
        finish = resolve;
      });
    });

    expect(find(handle.id)).toMatchObject({
      ...meta,
      status: 'running',
      progress: null,
    });

    report({ phase: 'java' });
    expect(find(handle.id)?.progress).toEqual({ phase: 'java' });

    finish('done-data');
    await expect(handle.result).resolves.toBe('done-data');
    expect(find(handle.id)).toMatchObject({
      status: 'done',
      progress: { phase: 'java' },
    });
    expect(find(handle.id)?.settledAt).not.toBeNull();
  });

  it('settles error with a HestiaError and rethrows', async () => {
    const handle = startJob(meta, () => Promise.reject(new Error('boom')));
    await expect(handle.result).rejects.toThrow('boom');
    const job = find(handle.id);
    expect(job?.status).toBe('error');
    expect(job?.error).toBeInstanceOf(HestiaError);
  });

  it('ignores progress reported after settling', async () => {
    let report: (progress: { phase: string }) => void = () => {};
    const handle = startJob<void, { phase: string }>(meta, (onProgress) => {
      report = onProgress;
      return Promise.resolve();
    });
    await handle.result;
    report({ phase: 'late' });
    expect(find(handle.id)?.progress).toBeNull();
  });

  it('dismisses settled jobs but never running ones', async () => {
    let finish: () => void = () => {};
    const running = startJob<void, never>(
      meta,
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const settled = startJob<void, never>(meta, () => Promise.resolve());
    await settled.result;

    dismissJob(running.id);
    expect(find(running.id)).toBeDefined();

    dismissJob(settled.id);
    expect(find(settled.id)).toBeUndefined();

    finish();
    await running.result;
    clearSettledJobs();
    expect(find(running.id)).toBeUndefined();
  });
});
