/**
 * Java runtimes: the installed and installable queries as named sub-results
 * (two independent fetches — flat-spreading both would tangle their states)
 * plus the install/uninstall actions.
 */
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import type { JavaInstallProgress } from '../api';
import { java } from '../api';
import { sweeper } from './client';
import { keys } from './keys';

export function useJava() {
  const queryClient = useQueryClient();
  const runtimes = useQuery({
    queryKey: keys.java.runtimes,
    queryFn: java.list,
  });
  const releases = useQuery({
    queryKey: keys.java.releases,
    queryFn: java.releases,
    staleTime: 60 * 60_000,
  });
  const actions = useMemo(() => {
    const done = sweeper(queryClient, keys.java.all);
    return {
      install: (
        major: number,
        onProgress?: (progress: JavaInstallProgress) => void,
      ) => done(java.install(major, {}, onProgress)),
      uninstall: (major: number) => done(java.uninstall(major)),
    };
  }, [queryClient]);
  return { runtimes, releases, ...actions };
}
