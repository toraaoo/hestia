/**
 * An instance's live sessions. One instance can be launched several times
 * concurrently; each launch is a supervised process keyed `instance-<id>_<seq>`
 * (`proto::naming`).
 */
import type { InstanceInfo, ProcessInfo } from '@/api';

const SESSION_SEQ = /_(\d+)$/;

/** The instance's running sessions, oldest launch first. */
export function runningSessions(instance: InstanceInfo): ProcessInfo[] {
  return (instance.sessions ?? [])
    .filter((session) => session.state === 'running')
    .sort((a, b) => a.startedUnix - b.startedUnix);
}

/**
 * The launch number the daemon gave a session. Read off the process key rather
 * than a list position, so a session keeps its name when an earlier one exits.
 */
export function sessionSeq(sessionId: string): number {
  const match = SESSION_SEQ.exec(sessionId);
  return match ? Number(match[1]) : 0;
}
