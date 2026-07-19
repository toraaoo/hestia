/**
 * The typed daemon API, one namespace per domain — the frontend's mirror of
 * the Rust client SDK's facades. Everything crosses the shell's generic
 * `ipc_call` bridge; channel names and payload shapes mirror `crates/proto`.
 */
export * as accounts from './accounts';
export * as app from './app';
export * as cache from './cache';
export * as config from './config';
export * as content from './content';
export {
  type ConnectionState,
  type DaemonEvent,
  onConnectionChange,
  onDaemonEvent,
  onTopic,
} from './core/events';
export {
  BAD_REQUEST,
  type CallOptions,
  CONNECTION_LOST,
  call,
  HANDLER_ERROR,
  HestiaError,
  isNotFound,
  NOT_FOUND,
  TIMEOUT,
  TRANSPORT,
  tryCall,
  UNKNOWN_CHANNEL,
} from './core/ipc';
export { type JobOptions, type JobTopics, jobId, runJob } from './core/jobs';
export * as daemon from './daemon';
export * as dialog from './dialog';
export * as download from './download';
export * as instance from './instance';
export * as java from './java';
export * as prefs from './prefs';
export * as process from './process';
export * as profile from './profile';
export * as server from './server';
export * as skins from './skins';
export * as sync from './sync';
export * as system from './system';
export type * from './types';
export { downgradeBetween } from './types/minecraft';
