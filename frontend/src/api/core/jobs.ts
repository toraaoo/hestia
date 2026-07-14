/**
 * The job driver, mirroring the client SDK's `Session::run_job`: generate a
 * job id, subscribe to it, start the job, then settle on its done/error
 * topic while streaming progress. The id is client-generated and subscribed
 * *before* the start call, so even a job that finishes instantly cannot slip
 * its terminal event past us.
 */
import type { ProvisionProgress } from '../types/minecraft';
import { onDaemonEvent } from './events';
import { call, HANDLER_ERROR, HestiaError } from './ipc';

let counter = 0;

export function jobId(prefix: string): string {
  counter += 1;
  return `${prefix}-${Date.now().toString(36)}-${counter}`;
}

export interface JobTopics {
  /** Progress topic; omit to forward every non-terminal event of the job. */
  progress?: string;
  done: string;
  error: string;
}

export interface JobOptions<TProgress = ProvisionProgress> {
  id: string;
  topics: JobTopics;
  onProgress?: (progress: TProgress) => void;
  /** The call that starts the job on the daemon. */
  start: () => Promise<unknown>;
}

/**
 * Run one daemon job to completion. Resolves with the done event's payload;
 * rejects with a `HestiaError` carrying the error event's message.
 */
export async function runJob<
  TDone extends { id: string },
  TProgress = ProvisionProgress,
>(options: JobOptions<TProgress>): Promise<TDone> {
  const { id, topics, onProgress } = options;
  let resolveOutcome!: (done: TDone) => void;
  let rejectOutcome!: (error: HestiaError) => void;
  const outcome = new Promise<TDone>((resolve, reject) => {
    resolveOutcome = resolve;
    rejectOutcome = reject;
  });

  let settled = false;
  const off = await onDaemonEvent((event) => {
    if (settled || event.payload.id !== id) return;
    if (event.topic === topics.done) {
      settled = true;
      resolveOutcome(event.payload as TDone);
    } else if (event.topic === topics.error) {
      settled = true;
      rejectOutcome(
        new HestiaError(HANDLER_ERROR, String(event.payload.message ?? '')),
      );
    } else if (
      onProgress &&
      (!topics.progress || event.topic === topics.progress)
    ) {
      onProgress(event.payload as unknown as TProgress);
    }
  });

  try {
    await call('events.subscribe', { id });
    await options.start();
    return await outcome;
  } finally {
    off();
  }
}
