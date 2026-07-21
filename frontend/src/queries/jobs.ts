/**
 * The global job store and the job-backed mutation factory. Every
 * long-running daemon operation (server create/update, instance launch,
 * backups, content installs, java installs, downloads) routes through
 * `startJob`, so an activity surface can render every in-flight job with
 * live progress no matter which component fired it — and the progress
 * outlives that component. `useJobMutation` adds the local view: the same
 * mutation result plus the `job`/`progress` of the run it started.
 */
import {
  type QueryKey,
  type UseMutationOptions,
  type UseMutationResult,
  useMutation,
} from '@tanstack/react-query';
import { useMemo, useState, useSyncExternalStore } from 'react';
import type { ProvisionProgress } from '../api';
import { HestiaError, TRANSPORT } from '../api';
import { invalidate } from './client';

export type JobEntryKind = 'server' | 'instance';

/** The entry a job acts on, when it acts on one. */
export interface JobEntry {
  kind: JobEntryKind;
  id: string;
}

export interface JobMeta {
  /** The operation, e.g. `server.create` — what an activity surface groups by. */
  kind: string;
  /** A short human fallback; the UI may localize from `kind`/`entry` instead. */
  label: string;
  entry?: JobEntry;
}

export type JobStatus = 'running' | 'done' | 'error';

export interface Job<TProgress = unknown> extends JobMeta {
  id: string;
  status: JobStatus;
  progress: TProgress | null;
  error: HestiaError | null;
  startedAt: number;
  settledAt: number | null;
  /** Shown in the status bar; a modal foregrounds its job to hide it there. */
  background: boolean;
}

const MAX_SETTLED = 50;

const jobs = new Map<string, Job>();
const listeners = new Set<() => void>();
let snapshot: Job[] = [];
let seq = 0;

function emit(): void {
  snapshot = [...jobs.values()];
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function patch(id: string, changes: Partial<Job>): void {
  const job = jobs.get(id);
  if (!job) return;
  jobs.set(id, { ...job, ...changes });
  emit();
}

function pruneSettled(): void {
  const settled = [...jobs.values()].filter((job) => job.status !== 'running');
  for (const job of settled.slice(
    0,
    Math.max(0, settled.length - MAX_SETTLED),
  )) {
    jobs.delete(job.id);
  }
}

function asHestiaError(error: unknown): HestiaError {
  return error instanceof HestiaError
    ? error
    : new HestiaError(TRANSPORT, String(error));
}

export interface JobHandle<TData> {
  id: string;
  result: Promise<TData>;
}

/** Run one tracked job: register it, stream its progress, settle its state. */
export function startJob<TData, TProgress>(
  meta: JobMeta,
  run: (onProgress: (progress: TProgress) => void) => Promise<TData>,
): JobHandle<TData> {
  seq += 1;
  const id = `job-${seq}`;
  jobs.set(id, {
    ...meta,
    id,
    status: 'running',
    progress: null,
    error: null,
    startedAt: Date.now(),
    settledAt: null,
    background: true,
  });
  pruneSettled();
  emit();

  const result = run((progress) => {
    if (jobs.get(id)?.status === 'running') patch(id, { progress });
  }).then(
    (data) => {
      patch(id, { status: 'done', settledAt: Date.now() });
      return data;
    },
    (error: unknown) => {
      patch(id, {
        status: 'error',
        error: asHestiaError(error),
        settledAt: Date.now(),
      });
      throw error;
    },
  );
  return { id, result };
}

/** Hide a job from the status bar while a modal owns its display. */
export function foregroundJob(id: string): void {
  if (jobs.get(id)?.background === false) return;
  patch(id, { background: false });
}

/** Reveal a job in the status bar after its modal is closed. */
export function backgroundJob(id: string): void {
  if (jobs.get(id)?.background === true) return;
  patch(id, { background: true });
}

/** Drop one settled job from the store; a running job stays. */
export function dismissJob(id: string): void {
  const job = jobs.get(id);
  if (!job || job.status === 'running') return;
  jobs.delete(id);
  emit();
}

export function clearSettledJobs(): void {
  let changed = false;
  for (const job of jobs.values()) {
    if (job.status === 'running') continue;
    jobs.delete(job.id);
    changed = true;
  }
  if (changed) emit();
}

/** The store's current jobs, oldest first — for non-React code and tests. */
export function getJobs(): Job[] {
  return snapshot;
}

/** Every tracked job, oldest first. */
export function useJobs(): Job[] {
  return useSyncExternalStore(subscribe, () => snapshot);
}

export function useJob(id: string | null): Job | null {
  return useSyncExternalStore(subscribe, () =>
    id ? (jobs.get(id) ?? null) : null,
  );
}

/** The jobs acting on one entry — e.g. a server card's busy indicator. */
export function useEntryJobs(kind: JobEntryKind, id: string): Job[] {
  const all = useJobs();
  return useMemo(
    () => all.filter((job) => job.entry?.kind === kind && job.entry.id === id),
    [all, kind, id],
  );
}

export interface JobSpec<TData, TVariables, TProgress> {
  mutationKey: QueryKey;
  meta: (variables: TVariables) => JobMeta;
  run: (
    variables: TVariables,
    onProgress: (progress: TProgress) => void,
  ) => Promise<TData>;
  /** Key prefixes swept when the job settles, success or error. */
  invalidates?: (variables: TVariables) => QueryKey[];
}

/**
 * `UseMutationOptions` that also carry the job descriptor, so plain
 * `useMutation` works (global tracking only) and `useJobMutation` can
 * additionally follow the run it starts.
 */
export type JobMutationOptions<TData, TVariables, TProgress> =
  UseMutationOptions<TData, HestiaError, TVariables> & {
    job: Pick<JobSpec<TData, TVariables, TProgress>, 'meta' | 'run'>;
  };

export function jobMutation<
  TData,
  TVariables = void,
  TProgress = ProvisionProgress,
>(
  spec: JobSpec<TData, TVariables, TProgress>,
): JobMutationOptions<TData, TVariables, TProgress> {
  return {
    mutationKey: spec.mutationKey,
    job: { meta: spec.meta, run: spec.run },
    mutationFn: (variables) =>
      startJob(spec.meta(variables), (onProgress: (p: TProgress) => void) =>
        spec.run(variables, onProgress),
      ).result,
    onSettled: (_data, _error, variables) => {
      for (const key of spec.invalidates?.(variables) ?? []) invalidate(key);
    },
  };
}

export type JobMutationResult<TData, TVariables, TProgress> = UseMutationResult<
  TData,
  HestiaError,
  TVariables
> & {
  /** The store's view of the run this hook instance started last. */
  job: Job<TProgress> | null;
  progress: TProgress | null;
};

/** `useMutation` plus live progress for the job this call site starts. */
export function useJobMutation<TData, TVariables, TProgress>(
  options: JobMutationOptions<TData, TVariables, TProgress>,
): JobMutationResult<TData, TVariables, TProgress> {
  const [jobRef, setJobRef] = useState<string | null>(null);
  const mutation = useMutation<TData, HestiaError, TVariables>({
    ...options,
    mutationFn: (variables) => {
      const handle = startJob<TData, TProgress>(
        options.job.meta(variables),
        (onProgress) => options.job.run(variables, onProgress),
      );
      setJobRef(handle.id);
      return handle.result;
    },
  });
  const job = useJob(jobRef) as Job<TProgress> | null;
  return { ...mutation, job, progress: job?.progress ?? null };
}
