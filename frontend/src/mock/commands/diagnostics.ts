/**
 * `log_write` and `crash_*` — the shell's log sink and crash-report store.
 * A browser has the devtools console for both, so the write is forwarded there
 * and the report store stays empty.
 */
import { type Handlers, str } from '../support';

export const commands: Handlers = {
  // The UI logs through pino as well; this is the file sink, which in the
  // browser is just the console at debug level.
  log_write: (p) => {
    console.debug(`[mock:${str(p, 'level', 'info')}]`, str(p, 'message'));
    return null;
  },

  crash_report: (p) => {
    console.error(`[mock:crash] ${str(p, 'kind')}`, str(p, 'message'));
    return null;
  },

  crash_list: () => [],
  crash_read: () => '',
  crash_clear: () => null,
};
