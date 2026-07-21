/** `config.*` — query/mutation factories, consumed through useQuery/useMutation. */
import { queryOptions } from '@tanstack/react-query';
import * as api from '../api/config';
import { mutation } from './core';
import { keys } from './keys';

export const configQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.config.list(),
      queryFn: () => api.list(),
    }),
  value: (key: string) =>
    queryOptions({
      queryKey: keys.config.value(key),
      queryFn: () => api.get(key),
    }),
};

export const configMutations = {
  set: () =>
    mutation<void, { key: string; value: unknown }>({
      mutationKey: [...keys.config.all, 'set'],
      mutationFn: ({ key, value }) => api.set(key, value),
      invalidates: () => [keys.config.all],
    }),
};
