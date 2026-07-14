/**
 * The hierarchical query-key factory. Invalidation targets a prefix
 * (`keys.servers.all` sweeps every server query), so keys must nest from
 * broad to narrow. Entries are keyed by their stable id — never the display
 * name, which a rename can change under a mounted component.
 */
import type { ContentKind, SearchQuery, VersionQuery } from '../api';

export const keys = {
  app: ['app'] as const,
  daemon: ['daemon'] as const,
  config: ['config'] as const,
  cache: ['cache'] as const,
  accounts: ['accounts'] as const,
  processes: ['processes'] as const,
  sync: ['sync'] as const,

  java: {
    all: ['java'] as const,
    releases: ['java', 'releases'] as const,
    runtimes: ['java', 'runtimes'] as const,
  },

  servers: {
    all: ['servers'] as const,
    list: ['servers', 'list'] as const,
    flavors: ['servers', 'flavors'] as const,
    versions: (flavor: string) => ['servers', 'versions', flavor] as const,
    detail: (id: string) => ['servers', 'detail', id] as const,
    logs: (id: string, tail?: number) =>
      ['servers', 'detail', id, 'logs', tail ?? null] as const,
    config: (id: string) => ['servers', 'detail', id, 'config'] as const,
    backups: (id: string) => ['servers', 'detail', id, 'backups'] as const,
    content: (id: string, kind: ContentKind) =>
      ['servers', 'detail', id, 'content', kind] as const,
  },

  instances: {
    all: ['instances'] as const,
    list: ['instances', 'list'] as const,
    flavors: ['instances', 'flavors'] as const,
    versions: (flavor: string) => ['instances', 'versions', flavor] as const,
    detail: (id: string) => ['instances', 'detail', id] as const,
    logs: (id: string, session?: string, tail?: number) =>
      [
        'instances',
        'detail',
        id,
        'logs',
        session ?? null,
        tail ?? null,
      ] as const,
    config: (id: string) => ['instances', 'detail', id, 'config'] as const,
    backups: (id: string) => ['instances', 'detail', id, 'backups'] as const,
    content: (id: string, kind: ContentKind) =>
      ['instances', 'detail', id, 'content', kind] as const,
    worlds: (id: string) => ['instances', 'detail', id, 'worlds'] as const,
  },

  content: {
    all: ['content'] as const,
    sources: ['content', 'sources'] as const,
    search: (query: SearchQuery) => ['content', 'search', query] as const,
    project: (source: string, project: string) =>
      ['content', 'project', source, project] as const,
    versions: (query: VersionQuery) => ['content', 'versions', query] as const,
    modpack: (source: string, versionId: string) =>
      ['content', 'modpack', source, versionId] as const,
  },
};
