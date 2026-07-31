/** `process.*` — thin over the fixture supervisor. */
import * as processes from '../state/processes';
import { fail, type Handlers, ok, str, strings } from '../support';

export const channels: Handlers = {
  'process.list': () => ({ processes: processes.list() }),

  'process.status': (p) => {
    const info = processes.get(str(p, 'id'));
    if (!info) fail('not_found', `no such process: ${str(p, 'id')}`);
    return info;
  },

  'process.logs': (p) => ({
    lines: processes.logs(str(p, 'id'), p.tail as number | undefined),
  }),

  'process.start': (p) => {
    const info = processes.start(
      str(p, 'id', `process-${Date.now().toString(36)}`),
      str(p, 'program'),
      strings(p, 'args'),
      [],
    );
    return { id: info.id, pid: info.pid };
  },

  'process.stop': (p) => {
    processes.stop(str(p, 'id'));
    return ok();
  },
};
