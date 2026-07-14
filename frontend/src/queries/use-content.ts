/** Content discovery hooks — Modrinth search, project detail, versions. */
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import type { SearchQuery, VersionQuery } from '../api';
import { content } from '../api';
import { keys } from './keys';

export function useContentSources() {
  return useQuery({
    queryKey: keys.content.sources,
    queryFn: content.sources,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

/** Paginated search; previous hits stay on screen while a page loads. */
export function useContentSearch(query: SearchQuery, enabled = true) {
  return useQuery({
    queryKey: keys.content.search(query),
    queryFn: () => content.search(query),
    placeholderData: keepPreviousData,
    staleTime: 60_000,
    enabled,
  });
}

export function useContentProject(projectId: string, source = '') {
  return useQuery({
    queryKey: keys.content.project(source, projectId),
    queryFn: () => content.project(projectId, source),
    enabled: projectId.length > 0,
    staleTime: 60_000,
  });
}

export function useContentVersions(query: VersionQuery, enabled = true) {
  return useQuery({
    queryKey: keys.content.versions(query),
    queryFn: () => content.versions(query),
    enabled: enabled && query.project.length > 0,
    staleTime: 60_000,
  });
}

/** Resolves a modpack version's file manifest (a slow inline download). */
export function useResolvedModpack(versionId: string, source = '') {
  return useQuery({
    queryKey: keys.content.modpack(source, versionId),
    queryFn: () => content.resolveModpack(versionId, source),
    enabled: versionId.length > 0,
    staleTime: Number.POSITIVE_INFINITY,
  });
}
