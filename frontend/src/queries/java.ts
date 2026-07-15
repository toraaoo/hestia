/** `java.*` — queries/mutations plus their 1:1 hooks. */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { JavaInstallDone, JavaInstallProgress } from '../api';
import * as api from '../api/java';
import { mutation } from './core';
import { jobMutation, useJobMutation } from './jobs';
import { keys } from './keys';

const CATALOG_STALE_MS = 5 * 60_000;

export const javaQueries = {
  releases: () =>
    queryOptions({
      queryKey: keys.java.releases(),
      queryFn: () => api.releases(),
      staleTime: CATALOG_STALE_MS,
    }),
  runtimes: () =>
    queryOptions({
      queryKey: keys.java.runtimes(),
      queryFn: () => api.list(),
    }),
};

export const javaMutations = {
  install: () =>
    jobMutation<
      JavaInstallDone,
      { major: number; force?: boolean },
      JavaInstallProgress
    >({
      mutationKey: [...keys.java.all, 'install'],
      meta: ({ major }) => ({
        kind: 'java.install',
        label: `install java ${major}`,
      }),
      run: ({ major, force }, onProgress) =>
        api.install(major, { force }, onProgress),
      invalidates: () => [keys.java.all],
    }),
  uninstall: () =>
    mutation<void, number>({
      mutationKey: [...keys.java.all, 'uninstall'],
      mutationFn: (major) => api.uninstall(major),
      invalidates: () => [keys.java.all],
    }),
};

export function useJavaReleases() {
  return useQuery(javaQueries.releases());
}

export function useJavaRuntimes() {
  return useQuery(javaQueries.runtimes());
}

export function useInstallJava() {
  return useJobMutation(javaMutations.install());
}

export function useUninstallJava() {
  return useMutation(javaMutations.uninstall());
}
