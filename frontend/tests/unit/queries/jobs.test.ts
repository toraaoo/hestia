import { afterEach, describe, expect, it } from 'vitest';
import { HestiaError } from '@/api';
import {
  backgroundJob,
  clearSettledJobs,
  dismissJob,
  foregroundJob,
  getJobs,
  type JobMeta,
  startJob,
} from '@/queries/jobs';

const meta = (over: Partial<JobMeta> = {}): JobMeta => ({
  kind: 'server.create',
  label: 'create',
  ...over,
});

const nextFrame = () =>
  new Promise((resolve) => requestAnimationFrame(resolve));

const find = (id: string) => getJobs().find((job) => job.id === id);

afterEach(clearSettledJobs);

describe('the job store', () => {
  it('registers a started job as running', () => {
    const { id } = startJob(meta(), () => Promise.resolve('ok'));
    expect(find(id)?.status).toBe('running');
    expect(find(id)?.kind).toBe('server.create');
  });

  it('mints the id and hands it to the run, surviving an await', async () => {
    let seen = '';
    const handle = startJob(meta(), async (job) => {
      await Promise.resolve();
      seen = job.id;
      return 'ok';
    });
    await handle.result;
    expect(seen).toBe(handle.id);
    expect(find(handle.id)).toBeDefined();
  });

  it('settles to done with the run’s value', async () => {
    const handle = startJob(meta(), () => Promise.resolve('landed'));
    await expect(handle.result).resolves.toBe('landed');
    expect(find(handle.id)?.status).toBe('done');
    expect(find(handle.id)?.settledAt).toBeTypeOf('number');
  });

  it('settles to error and keeps the daemon error', async () => {
    const failure = new HestiaError('handler_error', 'nope');
    const handle = startJob(meta(), () => Promise.reject(failure));
    await expect(handle.result).rejects.toBe(failure);
    expect(find(handle.id)?.status).toBe('error');
    expect(find(handle.id)?.error).toBe(failure);
  });

  it('wraps a non-daemon rejection so the store always has a code', async () => {
    const handle = startJob(meta(), () => Promise.reject(new Error('boom')));
    await expect(handle.result).rejects.toThrow('boom');
    expect(find(handle.id)?.error).toBeInstanceOf(HestiaError);
  });

  it('coalesces progress to one frame', async () => {
    let report!: (progress: number) => void;
    const handle = startJob<string, number>(meta(), (job) => {
      report = (progress) => job.onProgress?.(progress);
      return new Promise(() => {});
    });

    report(1);
    report(2);
    report(3);
    expect(find(handle.id)?.progress).toBeNull();

    await nextFrame();
    expect(find(handle.id)?.progress).toBe(3);
  });

  it('ignores progress reported after the job settled', async () => {
    let report!: (progress: number) => void;
    const handle = startJob<string, number>(meta(), (job) => {
      report = (progress) => job.onProgress?.(progress);
      return Promise.resolve('ok');
    });
    await handle.result;

    report(50);
    await nextFrame();
    expect(find(handle.id)?.progress).toBeNull();
    expect(find(handle.id)?.status).toBe('done');
  });

  it('tags the entry a job acts on', () => {
    const { id } = startJob(
      meta({ entry: { kind: 'instance', id: 'inst-1' } }),
      () => Promise.resolve('ok'),
    );
    expect(find(id)?.entry).toEqual({ kind: 'instance', id: 'inst-1' });
  });

  it('moves a job in and out of the status bar', () => {
    const { id } = startJob(meta(), () => Promise.resolve('ok'));
    expect(find(id)?.background).toBe(true);
    foregroundJob(id);
    expect(find(id)?.background).toBe(false);
    backgroundJob(id);
    expect(find(id)?.background).toBe(true);
  });

  it('dismisses a settled job but never a running one', async () => {
    const running = startJob(meta(), () => new Promise(() => {}));
    dismissJob(running.id);
    expect(find(running.id)).toBeDefined();

    const settled = startJob(meta(), () => Promise.resolve('ok'));
    await settled.result;
    dismissJob(settled.id);
    expect(find(settled.id)).toBeUndefined();
  });

  it('clears settled jobs and leaves running ones', async () => {
    const running = startJob(meta(), () => new Promise(() => {}));
    const settled = startJob(meta(), () => Promise.resolve('ok'));
    await settled.result;

    clearSettledJobs();
    expect(find(settled.id)).toBeUndefined();
    expect(find(running.id)?.status).toBe('running');
  });
});
