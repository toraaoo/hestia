/**
 * The shell's own two lifecycle commands: spawning the daemon (`ipc_call`
 * never does) and reading the archive the app was launched with.
 */
import { connection } from '../bus';
import { status } from '../channels/app';
import type { Handlers } from '../support';

export const commands: Handlers = {
  // The stop button reports `disconnected`; starting again is what clears it.
  start_daemon: () => {
    connection('connected');
    return status();
  },

  /** Null on an ordinary start — a browser is never handed a `.hestia` file. */
  pending_archive: () => null,
};
