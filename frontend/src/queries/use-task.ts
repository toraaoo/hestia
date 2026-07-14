/**
 * UI state over one bound action, for the components that want it: `run`
 * plus live `isPending` / `progress` / `error` / `data`. Actions are plain
 * async functions, so fire-and-forget call sites skip this hook entirely.
 *
 * Progress lights up automatically: the hook appends its own progress
 * callback when it calls the action, and every job-backed action accepts an
 * optional trailing `onProgress` parameter.
 */
import { useCallback, useRef, useState } from 'react';
import type { ProvisionProgress } from '../api';
import { HestiaError, TRANSPORT } from '../api';

export interface TaskState<TData, TProgress> {
  isPending: boolean;
  progress: TProgress | null;
  error: HestiaError | null;
  data: TData | undefined;
}

export interface Task<TArgs extends unknown[], TData, TProgress>
  extends TaskState<TData, TProgress> {
  run: (...args: TArgs) => Promise<TData>;
  reset: () => void;
}

const IDLE = {
  isPending: false,
  progress: null,
  error: null,
  data: undefined,
};

export function useTask<
  TArgs extends unknown[],
  TData,
  TProgress = ProvisionProgress,
>(
  action: (
    ...args: [...TArgs, ((progress: TProgress) => void)?]
  ) => Promise<TData>,
): Task<TArgs, TData, TProgress>;
export function useTask<TArgs extends unknown[], TData>(
  action: (...args: TArgs) => Promise<TData>,
): Task<TArgs, TData, never>;
export function useTask<
  TArgs extends unknown[],
  TData,
  TProgress = ProvisionProgress,
>(
  action: (
    ...args: [...TArgs, ((progress: TProgress) => void)?]
  ) => Promise<TData>,
): Task<TArgs, TData, TProgress> {
  const [state, setState] = useState<TaskState<TData, TProgress>>(IDLE);
  const actionRef = useRef(action);
  actionRef.current = action;
  // A re-run supersedes the previous one; a stale settle must not clobber it.
  const runSeq = useRef(0);

  const run = useCallback(async (...args: TArgs): Promise<TData> => {
    runSeq.current += 1;
    const seq = runSeq.current;
    const update = (next: TaskState<TData, TProgress>) => {
      if (seq === runSeq.current) setState(next);
    };

    update({ ...IDLE, isPending: true });
    try {
      const data = await actionRef.current(...args, (progress: TProgress) =>
        update({ ...IDLE, isPending: true, progress }),
      );
      update({ ...IDLE, data });
      return data;
    } catch (raw) {
      const error =
        raw instanceof HestiaError
          ? raw
          : new HestiaError(TRANSPORT, String(raw));
      update({ ...IDLE, error });
      throw error;
    }
  }, []);

  const reset = useCallback(() => {
    runSeq.current += 1;
    setState(IDLE);
  }, []);

  return { ...state, run, reset };
}
