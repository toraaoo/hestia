/**
 * The `content.*` browse channels — discovery on a source platform
 * (Modrinth). Installing into an entry lives on `server.content` /
 * `instance.content`. An empty `source` selects the default source.
 */
import { call } from './core/ipc';
import type {
  ContentProject,
  ContentSource,
  ContentVersion,
  ResolvedModpack,
  SearchQuery,
  SearchResult,
  VersionQuery,
} from './types/content';

export async function sources(): Promise<ContentSource[]> {
  const result = await call<{ sources: ContentSource[] }>('content.sources');
  return result.sources;
}

export function search(query: SearchQuery): Promise<SearchResult> {
  return call('content.search', query);
}

export function project(
  projectId: string,
  source = '',
): Promise<ContentProject> {
  return call('content.project', { source, project: projectId });
}

export async function versions(query: VersionQuery): Promise<ContentVersion[]> {
  const result = await call<{ versions: ContentVersion[] }>(
    'content.versions',
    query,
  );
  return result.versions;
}

/** Downloads and reads the `.mrpack` index inline — hence the long timeout. */
export function resolveModpack(
  versionId: string,
  source = '',
): Promise<ResolvedModpack> {
  return call(
    'content.modpack.resolve',
    { source, versionId },
    { timeoutMs: 120_000 },
  );
}
