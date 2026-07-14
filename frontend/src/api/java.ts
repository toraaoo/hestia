/** The `java.*` channels. */

import { call } from './core/ipc';
import { jobId, runJob } from './core/jobs';
import type {
  JavaInstallDone,
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
  options: { force?: boolean } = {},
  onProgress?: (progress: JavaInstallProgress) => void,
): Promise<JavaInstallDone> {
  const id = jobId('java-install');
  return runJob<JavaInstallDone, JavaInstallProgress>({
    id,
    topics: {
      progress: 'java.install.progress',
      done: 'java.install.done',
      error: 'java.install.error',
    },
    onProgress,
    start: () =>
      call('java.install', { major, id, force: options.force ?? false }),
  });
}

export async function uninstall(major: number): Promise<void> {
  await call('java.uninstall', { major });
}
