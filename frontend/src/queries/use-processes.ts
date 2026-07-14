/** Raw supervisor hooks; servers and instances have richer wrappers. */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { process } from '../api';
import { sweeper } from './client';
import { keys } from './keys';

export function useProcesses() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: keys.processes, queryFn: process.list });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.processes);
    return {
      stop: (id: string) => done(process.stop(id)),
    };
  }, [queryClient]);
  return { ...query, ...actions };
}
