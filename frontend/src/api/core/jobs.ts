/**
 * The job driver, mirroring the client SDK's `Session::run_job`: generate a
 * job id, listen for its events, start the job, then settle on its done/error
 * topic while streaming progress. The event handler is installed *before* the
 * start call, so even a job that finishes instantly cannot slip its terminal
 * event past us. No per-job `events.subscribe` is issued: the desktop bridge
 * already holds one connection subscribed to every daemon event, so a second
 * subscription would only duplicate delivery (and leak a daemon-side sub that
 * is pruned only on disconnect).
 */
import { logger } from '@/lib/log';
import type { ErrorInfo } from '../types/error';
import type { ProvisionProgress } from '../types/minecraft';
import { onDaemonEvent } from './events';
import { HANDLER_ERROR, HestiaError } from './ipc';

const log = logger('job');

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

/**
 * The rejection a cancelled job settles with. Distinguished from a failure so a
 * surface can say "cancelled" instead of rendering an error the user caused on
 * purpose.
 */
export class JobCancelled extends Error {
  constructor(readonly id: string) {
    super('cancelled');
    this.name = 'JobCancelled';
  }
}

/**
 * Every job family names its terminal topics `<family>.done|error|cancelled`,
 * so the third is derived rather than declared at each of the call sites.
 */
function cancelledTopic(done: string): string {
  return done.replace(/\.done$/, '.cancelled');
}

/**
 * One run's identity, minted by the caller and threaded into the API function.
 * Passed in rather than reported back so the caller's tracking cannot be
 * orphaned by an `await` on the way to `runJob`.
 */
export interface JobRun<TProgress = ProvisionProgress> {
  id: string;
  onProgress?: (progress: TProgress) => void;
}

export interface JobOptions<TProgress = ProvisionProgress>
  extends JobRun<TProgress> {
  topics: JobTopics;
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

  const cancelled = cancelledTopic(topics.done);
  let settled = false;
  const off = await onDaemonEvent((event) => {
    if (settled || event.payload.id !== id) return;
    if (event.topic === topics.done) {
      settled = true;
      resolveOutcome(event.payload as TDone);
    } else if (event.topic === cancelled) {
      settled = true;
      log.debug({ id }, 'job cancelled');
      rejectOutcome(new JobCancelled(id) as unknown as HestiaError);
    } else if (event.topic === topics.error) {
      settled = true;
      // Job error events carry the structured `ErrorInfo`; the display site
      // localizes it. `kind` is a stable fallback message, shown only if that
      // localization ever misses.
      const info = (event.payload.error ?? null) as ErrorInfo | null;
      log.warn({ id, kind: info?.kind }, 'job failed');
      rejectOutcome(
        new HestiaError(HANDLER_ERROR, info?.kind ?? 'job failed', info),
      );
    } else if (
      onProgress &&
      (!topics.progress || event.topic === topics.progress)
    ) {
      onProgress(event.payload as unknown as TProgress);
    }
  });

  try {
    log.debug({ id, done: topics.done }, 'job started');
    await options.start();
    return await outcome;
  } finally {
    off();
  }
}
