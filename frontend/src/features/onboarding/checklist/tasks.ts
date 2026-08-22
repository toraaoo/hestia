import type { InstanceInfo, ServerInfo } from '@/api';
import { m } from '@/paraglide/messages.js';

export interface ChecklistContext {
  signedIn: boolean;
  instances: InstanceInfo[];
  servers: ServerInfo[];
}

export type TaskAction =
  | { kind: 'sign-in' }
  | { kind: 'create'; entry: 'instance' | 'server' }
  | { kind: 'play' };

export interface ChecklistTask {
  id: string;
  label: () => string;
  hint: () => string;
  action: TaskAction;
  done: (context: ChecklistContext) => boolean;
  blocked?: (context: ChecklistContext) => boolean;
}

export const tasks: readonly ChecklistTask[] = [
  {
    id: 'sign-in',
    label: m['onboarding.checklist.sign_in.label'],
    hint: m['onboarding.checklist.sign_in.hint'],
    action: { kind: 'sign-in' },
    done: ({ signedIn }) => signedIn,
  },
  {
    id: 'instance',
    label: m['onboarding.checklist.instance.label'],
    hint: m['onboarding.checklist.instance.hint'],
    action: { kind: 'create', entry: 'instance' },
    done: ({ instances }) => instances.length > 0,
    blocked: ({ signedIn }) => !signedIn,
  },
  {
    id: 'play',
    label: m['onboarding.checklist.play.label'],
    hint: m['onboarding.checklist.play.hint'],
    action: { kind: 'play' },
    done: ({ instances }) => instances.some((i) => Boolean(i.lastPlayedUnix)),
    blocked: ({ instances }) => instances.length === 0,
  },
  {
    id: 'server',
    label: m['onboarding.checklist.server.label'],
    hint: m['onboarding.checklist.server.hint'],
    action: { kind: 'create', entry: 'server' },
    done: ({ servers }) => servers.length > 0,
  },
];

export interface TaskProgress {
  task: ChecklistTask;
  done: boolean;
  blocked: boolean;
}

export function progress(context: ChecklistContext): TaskProgress[] {
  return tasks.map((task) => ({
    task,
    done: task.done(context),
    blocked: task.blocked?.(context) ?? false,
  }));
}
