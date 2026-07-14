/** Mirrors `crates/proto/src/instance.rs`. */
import type { ConfigEntry } from './minecraft';
import type { ProcessInfo } from './process';

/**
 * A managed instance: the stored record plus, when launched, its live
 * sessions — an instance can run more than once concurrently.
 */
export interface InstanceInfo {
  id: string;
  name: string;
  flavor: string;
  game_version: string;
  loader_version?: string;
  java_major: number;
  created_unix: number;
  sessions?: ProcessInfo[];
}

export interface InstanceCreateParams {
  /** Display name; defaults to `<flavor>-<version>` when empty. */
  name?: string;
  flavor: string;
  version: string;
  loader_version?: string;
  /** Create-time settings (memory, jvm-args). */
  config?: ConfigEntry[];
}

export interface InstanceUpdateParams {
  /** Instance name or id. */
  instance: string;
  version: string;
  loader_version?: string;
  /** The caller confirms the risk of moving to an older version. */
  allow_downgrade?: boolean;
}

export interface InstanceLaunchParams {
  instance: string;
  /** Account name or uuid; empty picks the sole signed-in account. */
  account?: string;
  /** Launch another session even when one is already running. */
  new_session?: boolean;
}

export interface InstanceLaunchDone {
  id: string;
  process_id: string;
  pid: number;
}
