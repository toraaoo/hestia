/**
 * The hierarchical query-key factory. Every cached read gets its key here so
 * invalidation can sweep by prefix: a kind's `all` covers its lists and every
 * entry, and an entry's per-resource keys nest under `detail(id)` so one
 * sweep refreshes the whole entry. Entries are keyed by their **stable id**
 * (never the renameable display name), so a rename cannot strand a cache key.
 */
import type {
  ContentKind,
  ResolveParams,
  SearchQuery,
  VersionQuery,
} from '../api';

export const keys = {
  app: {
    all: ['app'] as const,
    info: () => [...keys.app.all, 'info'] as const,
    ping: () => [...keys.app.all, 'ping'] as const,
  },
  daemon: {
    all: ['daemon'] as const,
    status: () => [...keys.daemon.all, 'status'] as const,
  },
  config: {
    all: ['config'] as const,
    list: () => [...keys.config.all, 'list'] as const,
    value: (key: string) => [...keys.config.all, 'value', key] as const,
  },
  prefs: {
    all: ['prefs'] as const,
    list: () => [...keys.prefs.all, 'list'] as const,
  },
  cache: {
    all: ['cache'] as const,
    info: () => [...keys.cache.all, 'info'] as const,
    list: () => [...keys.cache.all, 'list'] as const,
  },
  accounts: {
    all: ['accounts'] as const,
    list: () => [...keys.accounts.all, 'list'] as const,
  },
  java: {
    all: ['java'] as const,
    releases: () => [...keys.java.all, 'releases'] as const,
    runtimes: () => [...keys.java.all, 'runtimes'] as const,
  },
  processes: {
    all: ['processes'] as const,
    list: () => [...keys.processes.all, 'list'] as const,
    status: (id: string) => [...keys.processes.all, 'status', id] as const,
    logs: (id: string, tail?: number) =>
      [...keys.processes.all, 'logs', id, tail ?? null] as const,
  },
  servers: {
    all: ['servers'] as const,
    list: () => [...keys.servers.all, 'list'] as const,
    flavors: () => [...keys.servers.all, 'flavors'] as const,
    versions: (flavor: string) =>
      [...keys.servers.all, 'versions', flavor] as const,
    loaders: (flavor: string, version: string) =>
      [...keys.servers.all, 'loaders', flavor, version] as const,
    profile: (params: ResolveParams) =>
      [...keys.servers.all, 'profile', params] as const,
    detail: (id: string) => [...keys.servers.all, 'detail', id] as const,
    ping: (id: string) => [...keys.servers.detail(id), 'ping'] as const,
    logs: (id: string, tail?: number) =>
      [...keys.servers.detail(id), 'logs', tail ?? null] as const,
    config: (id: string) => [...keys.servers.detail(id), 'config'] as const,
    configValue: (id: string, key: string) =>
      [...keys.servers.config(id), key] as const,
    backups: (id: string) => [...keys.servers.detail(id), 'backups'] as const,
    content: (id: string) => [...keys.servers.detail(id), 'content'] as const,
    contentList: (id: string, kind: ContentKind) =>
      [...keys.servers.content(id), kind] as const,
  },
  instances: {
    all: ['instances'] as const,
    list: () => [...keys.instances.all, 'list'] as const,
    flavors: () => [...keys.instances.all, 'flavors'] as const,
    versions: (flavor: string) =>
      [...keys.instances.all, 'versions', flavor] as const,
    loaders: (flavor: string, version: string) =>
      [...keys.instances.all, 'loaders', flavor, version] as const,
    profile: (params: ResolveParams) =>
      [...keys.instances.all, 'profile', params] as const,
    detail: (id: string) => [...keys.instances.all, 'detail', id] as const,
    worlds: (id: string) => [...keys.instances.detail(id), 'worlds'] as const,
    logs: (id: string, session?: string, tail?: number) =>
      [
        ...keys.instances.detail(id),
        'logs',
        session ?? null,
        tail ?? null,
      ] as const,
    config: (id: string) => [...keys.instances.detail(id), 'config'] as const,
    configValue: (id: string, key: string) =>
      [...keys.instances.config(id), key] as const,
    content: (id: string) => [...keys.instances.detail(id), 'content'] as const,
    contentList: (id: string, kind: ContentKind) =>
      [...keys.instances.content(id), kind] as const,
    profiles: (id: string) =>
      [...keys.instances.detail(id), 'profiles'] as const,
  },
  profiles: {
    all: ['profiles'] as const,
    list: () => [...keys.profiles.all, 'list'] as const,
  },
  content: {
    all: ['content'] as const,
    sources: () => [...keys.content.all, 'sources'] as const,
    search: (query: SearchQuery) =>
      [...keys.content.all, 'search', query] as const,
    project: (source: string, project: string) =>
      [...keys.content.all, 'project', source, project] as const,
    versions: (query: VersionQuery) =>
      [...keys.content.all, 'versions', query] as const,
    modpack: (source: string, versionId: string) =>
      [...keys.content.all, 'modpack', source, versionId] as const,
  },
  skins: {
    all: ['skins'] as const,
    list: (account: string) => [...keys.skins.all, 'list', account] as const,
  },
  sync: {
    all: ['sync'] as const,
    config: () => [...keys.sync.all, 'config'] as const,
    status: () => [...keys.sync.all, 'status'] as const,
  },
};
