/**
 * `download.start` — a URL streamed to a daemon-local path. A job with its own
 * byte-counting progress shape.
 */
import { jobIdOf, runPlan } from '../job';
import { type Handlers, str } from '../support';

const TOTAL = 24 * 1024 * 1024;
const TICKS = 6;

export const channels: Handlers = {
  'download.start': (p) => {
    const dest = str(p, 'dest', '/mock/downloads/file.bin');
    return runPlan({
      id: jobIdOf(p, 'download'),
      family: 'download',
      ticks: Array.from({ length: TICKS }, (_, index) => ({
        downloaded: Math.round((TOTAL / TICKS) * (index + 1)),
        total: TOTAL,
      })),
      done: () => ({ path: dest }),
    });
  },
};
