import { afterEach, describe, expect, it, vi } from 'vitest';
import { HestiaError, JobCancelled, jobId, runJob } from '@/api';
import { publish } from '@/mock/bus';
import { installFakeDaemon } from '../../support';

installFakeDaemon();

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

afterEach(() => vi.restoreAllMocks());

const topics = {
  progress: 'server.create.progress',
  done: 'server.create.done',
  error: 'server.create.error',
};

describe('runJob', () => {
  it('resolves with the done event of its own job', async () => {
    const id = jobId('server.create');
    const outcome = runJob<{ id: string; server: string }>({
      id,
      topics,
      start: async () => {
        await settle();
        publish('server.create.done', { id, server: 'smp' });
      },
    });
    await expect(outcome).resolves.toMatchObject({ id, server: 'smp' });
  });

  it('ignores events belonging to another job', async () => {
    const id = jobId('server.create');
    const outcome = runJob<{ id: string; server: string }>({
      id,
      topics,
      start: async () => {
        await settle();
        publish('server.create.done', { id: 'someone-else', server: 'other' });
        publish('server.create.done', { id, server: 'mine' });
      },
    });
    await expect(outcome).resolves.toMatchObject({ server: 'mine' });
  });

  it('streams progress until it settles', async () => {
    const id = jobId('server.create');
    const seen: number[] = [];
    await runJob<{ id: string }, { percent: number }>({
      id,
      topics,
      onProgress: (p) => seen.push(p.percent),
      start: async () => {
        await settle();
        publish('server.create.progress', { id, percent: 10 });
        publish('server.create.progress', { id, percent: 90 });
        publish('server.create.done', { id });
      },
    });
    expect(seen).toEqual([10, 90]);
  });

  it('stops reporting progress once the job is done', async () => {
    const id = jobId('server.create');
    const seen: number[] = [];
    await runJob<{ id: string }, { percent: number }>({
      id,
      topics,
      onProgress: (p) => seen.push(p.percent),
      start: async () => {
        await settle();
        publish('server.create.done', { id });
        publish('server.create.progress', { id, percent: 100 });
      },
    });
    await settle();
    expect(seen).toEqual([]);
  });

  it('rejects with the structured error the daemon reported', async () => {
    const id = jobId('server.create');
    const outcome = runJob<{ id: string }>({
      id,
      topics,
      start: async () => {
        await settle();
        publish('server.create.error', {
          id,
          error: { kind: 'already_exists', entry: 'server', name: 'smp' },
        });
      },
    });
    await expect(outcome).rejects.toBeInstanceOf(HestiaError);
    await outcome.catch((error: HestiaError) => {
      expect(error.info?.kind).toBe('already_exists');
    });
  });

  it('distinguishes a cancellation from a failure', async () => {
    const id = jobId('server.create');
    const outcome = runJob<{ id: string }>({
      id,
      topics,
      start: async () => {
        await settle();
        publish('server.create.cancelled', { id });
      },
    });
    await expect(outcome).rejects.toBeInstanceOf(JobCancelled);
  });

  it('propagates a rejection from the start call itself', async () => {
    const id = jobId('server.create');
    await expect(
      runJob<{ id: string }>({
        id,
        topics,
        start: () => Promise.reject(new HestiaError('bad_request', 'nope')),
      }),
    ).rejects.toThrow('nope');
  });

  it('hears a terminal event that arrives before start resolves', async () => {
    const id = jobId('server.create');
    const outcome = runJob<{ id: string; server: string }>({
      id,
      topics,
      start: async () => {
        publish('server.create.done', { id, server: 'instant' });
        await settle();
      },
    });
    await expect(outcome).resolves.toMatchObject({ server: 'instant' });
  });
});
