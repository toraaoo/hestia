/**
 * `content.*` browse — discovery on a source platform (queries only;
 * installing into an entry lives on the server/instance hooks). An empty
 * `source` selects the default source.
 */
import {
  keepPreviousData,
  queryOptions,
  useQuery,
} from '@tanstack/react-query';
import type { SearchQuery, VersionQuery } from '../api';
import * as api from '../api/content';
import { keys } from './keys';

const BROWSE_STALE_MS = 60_000;

export const contentQueries = {
  sources: () =>
    queryOptions({
      queryKey: keys.content.sources(),
      queryFn: () => api.sources(),
      staleTime: Number.POSITIVE_INFINITY,
    }),
  search: (query: SearchQuery) =>
    queryOptions({
      queryKey: keys.content.search(query),
      queryFn: () => api.search(query),
      staleTime: BROWSE_STALE_MS,
      // Keep the current hits on screen while a larger page (or a new filter)
      // loads, so paginating never flashes the grid back to a skeleton.
      placeholderData: keepPreviousData,
    }),
  project: (project: string, source = '') =>
    queryOptions({
      queryKey: keys.content.project(source, project),
      queryFn: () => api.project(project, source),
      staleTime: BROWSE_STALE_MS,
    }),
  versions: (query: VersionQuery) =>
    queryOptions({
      queryKey: keys.content.versions(query),
      queryFn: () => api.versions(query),
      staleTime: BROWSE_STALE_MS,
    }),
  /** Downloads and reads the `.mrpack` index — mount deliberately. */
  modpack: (versionId: string, source = '') =>
    queryOptions({
      queryKey: keys.content.modpack(source, versionId),
      queryFn: () => api.resolveModpack(versionId, source),
      staleTime: BROWSE_STALE_MS,
    }),
};

export function useContentSources() {
  return useQuery(contentQueries.sources());
}

export function useContentSearch(query: SearchQuery) {
  return useQuery(contentQueries.search(query));
}

export function useContentProject(project: string, source = '') {
  return useQuery(contentQueries.project(project, source));
}

export function useContentVersions(query: VersionQuery) {
  return useQuery(contentQueries.versions(query));
}

export function useResolvedModpack(versionId: string, source = '') {
  return useQuery(contentQueries.modpack(versionId, source));
}
