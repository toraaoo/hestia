/**
 * `job.cancel` — one channel for every job family, since a job is identified
 * by the id its own events carry.
 */
import { cancelJob } from '../job';
import { type Handlers, str } from '../support';

export const channels: Handlers = {
  'job.cancel': (p) => ({ cancelled: cancelJob(str(p, 'id')) }),
};
