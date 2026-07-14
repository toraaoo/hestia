/**
 * React Query bindings over the daemon API: the query client, the
 * hierarchical key factory, event-driven invalidation, and entity-scoped
 * hooks — one hook returns an entry's query state spread with its bound
 * actions, `useTask` adds pending/progress UI state where a component
 * wants it.
 */
export { createQueryClient } from './client';
export { installDaemonInvalidation } from './invalidation';
export { keys } from './keys';
export * from './use-accounts';
export * from './use-app';
export * from './use-content';
export * from './use-events';
export * from './use-instances';
export * from './use-java';
export * from './use-processes';
export * from './use-servers';
export { type Task, type TaskState, useTask } from './use-task';
