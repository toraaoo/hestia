import { screen, waitFor } from '@testing-library/react';
import { act } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { jobMutation, useJobMutation } from '@/queries/jobs';
import { renderWithProviders, resetJobs, resetQueryCache } from '../../support';

afterEach(async () => {
  await resetJobs();
  resetQueryCache();
});

const received: string[] = [];
let report: ((progress: number) => void) | null = null;
let finish: ((value: string) => void) | null = null;

const spec = jobMutation<string, void, number>({
  mutationKey: ['test', 'job'],
  meta: () => ({ kind: 'server.create', label: 'create' }),
  run: (_variables, job) => {
    received.push(job.id);
    report = (progress) => job.onProgress?.(progress);
    return new Promise<string>((resolve) => {
      finish = resolve;
    });
  },
});

function Harness() {
  const run = useJobMutation(spec);
  return (
    <div>
      <button type="button" onClick={() => run.mutate()}>
        go
      </button>
      <span data-testid="id">{run.job?.id ?? '-'}</span>
      <span data-testid="progress">{run.progress ?? '-'}</span>
      <span data-testid="status">{run.job?.status ?? '-'}</span>
    </div>
  );
}

describe('useJobMutation', () => {
  it('tracks the run it started and streams its progress', async () => {
    renderWithProviders(<Harness />);
    screen.getByText('go').click();

    await waitFor(() => expect(received).toHaveLength(1));
    await waitFor(() =>
      expect(screen.getByTestId('id').textContent).toBe(received[0]),
    );

    act(() => report?.(42));
    await waitFor(() =>
      expect(screen.getByTestId('progress').textContent).toBe('42'),
    );

    await act(async () => {
      finish?.('done');
      await Promise.resolve();
    });
    await waitFor(() =>
      expect(screen.getByTestId('status').textContent).toBe('done'),
    );
  });
});
