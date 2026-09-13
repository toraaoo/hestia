/**
 * `screenshot.*` — the aggregated listing and deleting one where it lies.
 */
import { queryOptions } from '@tanstack/react-query';

import type { Screenshot } from '../api';
import * as api from '../api/screenshot';
import { mutation } from './core';
import { keys } from './keys';

export const screenshotQueries = {
  list: (instance = '') =>
    queryOptions({
      queryKey: keys.screenshots.list(instance),
      queryFn: () => api.list(instance),
    }),
};

export const screenshotMutations = {
  remove: () =>
    mutation<Screenshot[], { instance: string; file: string }>({
      mutationKey: [...keys.screenshots.all, 'remove'],
      mutationFn: ({ instance, file }) => api.remove(instance, file),
      invalidates: () => [keys.screenshots.all],
    }),
};
