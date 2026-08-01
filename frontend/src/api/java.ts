/** The `java.*` channels. */

import { call } from './core/ipc';
import { type JobRun, runJob } from './core/jobs';
import type {
  JavaInstallDoneEvent,
  JavaInstallProgress,
  JavaRelease,
  JavaRuntime,
} from './types/java';

export async function releases(): Promise<JavaRelease[]> {
  const result = await call<{ releases: JavaRelease[] }>('java.releases');
  return result.releases;
}

export async function list(): Promise<JavaRuntime[]> {
  const result = await call<{ runtimes: JavaRuntime[] }>('java.list');
  return result.runtimes;
}

export function install(
  major: number,
  options: { force?: boolean },
  job: JobRun<JavaInstallProgress>,
): Promise<JavaInstallDoneEvent> {
  return runJob<JavaInstallDoneEvent, JavaInstallProgress>({
    ...job,
    topics: {
      progress: 'java.install.progress',
      done: 'java.install.done',
      error: 'java.install.error',
    },
    start: () =>
      call('java.install', {
        major,
        id: job.id,
        force: options.force ?? false,
      }),
  });
}

export async function uninstall(major: number): Promise<void> {
  await call('java.uninstall', { major });
}
