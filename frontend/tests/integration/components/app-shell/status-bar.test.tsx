import { screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { StatusBar } from '@/components/app-shell/status-bar';
import { status as daemonStatus } from '@/mock/channels/app';
import { listServers } from '@/mock/state/entries';
import { m } from '@/paraglide/messages.js';
import { foregroundJob, type JobMeta, startJob } from '@/queries/jobs';
import {
  pendingJob,
  renderWithProviders,
  resetJobs,
  resetQueryCache,
} from '../../../support';

const meta = (over: Partial<JobMeta> = {}): JobMeta => ({
  kind: 'server.create',
  label: 'create smp',
  ...over,
});

afterEach(async () => {
  await resetJobs();
  resetQueryCache();
});

describe('the status bar', () => {
  it('reports the daemon it is talking to', async () => {
    renderWithProviders(<StatusBar />);
    expect(screen.getByText(m['app.daemon.connected']())).toBeDefined();
    await waitFor(() =>
      expect(
        screen.getByText(`v${daemonStatus().version}`),
      ).toBeDefined(),
    );
  });

  it('shows nothing about jobs while none are running', () => {
    renderWithProviders(<StatusBar />);
    expect(screen.queryByText('create smp')).toBeNull();
  });

  it('surfaces a running background job by its label', async () => {
    startJob(meta(), pendingJob);
    renderWithProviders(<StatusBar />);
    await waitFor(() => expect(screen.getByText('create smp')).toBeDefined());
  });

  it('prefers the entry’s live name over the job label', async () => {
    const server = listServers()[0];
    startJob(meta({ entry: { kind: 'server', id: server.id } }), pendingJob);

    renderWithProviders(<StatusBar />);
    await waitFor(() => expect(screen.getByText(server.name)).toBeDefined());
    expect(screen.queryByText('create smp')).toBeNull();
  });

  it('counts the jobs beyond the one shown inline', async () => {
    startJob(meta(), pendingJob);
    startJob(meta({ label: 'create creative' }), pendingJob);
    renderWithProviders(<StatusBar />);
    await waitFor(() =>
      expect(screen.getByText(m['app.jobs.more']({ count: 1 }))).toBeDefined(),
    );
  });

  it('hides a job a modal has taken into the foreground', async () => {
    const { id } = startJob(meta(), pendingJob);
    renderWithProviders(<StatusBar />);
    await waitFor(() => expect(screen.getByText('create smp')).toBeDefined());

    foregroundJob(id);
    await waitFor(() => expect(screen.queryByText('create smp')).toBeNull());
  });

  it('drops a job from the bar once it settles', async () => {
    const handle = startJob(meta(), () => Promise.resolve('ok'));
    renderWithProviders(<StatusBar />);
    await handle.result;
    await waitFor(() => expect(screen.queryByText('create smp')).toBeNull());
  });
});
