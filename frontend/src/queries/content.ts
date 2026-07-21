/**
 * `content.*` browse — discovery on a source platform (queries only;
 * installing into an entry lives on the server/instance hooks). An empty
 * `source` selects the default source.
 */
import {
  infiniteQueryOptions,
  keepPreviousData,
  queryOptions,
  useInfiniteQuery,
  useQuery,
} from '@tanstack/react-query';
import type { ContentKind, SearchQuery, VersionQuery } from '../api';
import * as api from '../api/content';
import type { QueryFlags } from './core';
import { keys } from './keys';

const BROWSE_STALE_MS = 60_000;

/** Hits fetched per page when paging the browse grid. */
export const SEARCH_PAGE = 20;

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
      // A single observer transitions keys cleanly, so keep the current hits on
      // screen while a new query loads (search-as-you-type never blanks).
      placeholderData: keepPreviousData,
    }),
  // The browse grid pages by offset: pages accumulate under one stable key, so
  // scrolling appends rather than refetching a growing window. "All" fans out
  // over every kind for a page (a source search is scoped to one kind) and the
  // grid merges them. `keepPreviousData` keeps the old results on screen while
  // a new filter loads.
  searchPaged: (kinds: ContentKind[], query: string) =>
    infiniteQueryOptions({
      queryKey: keys.content.searchPaged(kinds, query),
      queryFn: ({ pageParam }) =>
        Promise.all(
          kinds.map((kind) =>
            api.search({ kind, query, limit: SEARCH_PAGE, offset: pageParam }),
          ),
        ),
      initialPageParam: 0,
      getNextPageParam: (lastPage, _pages, lastParam) =>
        lastPage.some((r) => r.offset + r.hits.length < r.total)
          ? lastParam + SEARCH_PAGE
          : undefined,
      staleTime: BROWSE_STALE_MS,
      placeholderData: keepPreviousData,
    }),
  project: (project: string, source = '') =>
    queryOptions({
      queryKey: keys.content.project(source, project),
      queryFn: () => api.project(project, source),
      enabled: project.length > 0,
      staleTime: BROWSE_STALE_MS,
    }),
  versions: (query: VersionQuery) =>
    queryOptions({
      queryKey: keys.content.versions(query),
      queryFn: () => api.versions(query),
      enabled: query.project.length > 0,
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

export function useContentSearchPaged(kinds: ContentKind[], query: string) {
  return useInfiniteQuery(contentQueries.searchPaged(kinds, query));
}

export function useContentProject(project: string, source = '') {
  return useQuery(contentQueries.project(project, source));
}

export function useContentVersions(
  query: VersionQuery,
  { enabled = true }: QueryFlags = {},
) {
  const options = contentQueries.versions(query);
  return useQuery({ ...options, enabled: enabled && options.enabled });
}

export function useResolvedModpack(versionId: string, source = '') {
  return useQuery(contentQueries.modpack(versionId, source));
}
