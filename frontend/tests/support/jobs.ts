import { clearSettledJobs } from '@/queries/jobs';

const open: Array<() => void> = [];

export function pendingJob<T = void>(): Promise<T> {
  return new Promise<T>((resolve) => {
    open.push(() => resolve(undefined as T));
  });
}

export async function resetJobs(): Promise<void> {
  for (const settle of open) settle();
  open.length = 0;
  await Promise.resolve();
  await Promise.resolve();
  clearSettledJobs();
}
