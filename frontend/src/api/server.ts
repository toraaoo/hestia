/**
 * The `server.*` channels. Every per-entry call names the server by display
 * name or id (`proto::naming` resolves either). `create` and `update` are
 * jobs — the returned promise settles on the job's terminal event while
 * progress streams through the callback; backups and content installs share
 * the `backup.*` / `content.*` topics, disambiguated by job id.
 */

import { call, tryCall } from './core/ipc';
import { jobId, runJob } from './core/jobs';
import type { BackupInfo } from './types/backup';
import type {
  ContentAddSpec,
  ContentDone,
  ContentKind,
  ContentList,
  ContentUpdate,
} from './types/content';
import type {
  ConfigEntry,
  Flavor,
  GameVersion,
  ProvisionProgress,
  ResolveParams,
  ServerProfile,
} from './types/minecraft';
import type { ProcessLogLine } from './types/process';
import type {
  ServerCreateParams,
  ServerDetails,
  ServerInfo,
  ServerPingResult,
  ServerUpdateParams,
} from './types/server';

type OnProgress = (progress: ProvisionProgress) => void;

export async function flavors(): Promise<Flavor[]> {
  const result = await call<{ flavors: Flavor[] }>('server.flavors');
  return result.flavors;
}

export async function versions(flavor: string): Promise<GameVersion[]> {
  const result = await call<{ versions: GameVersion[] }>('server.versions', {
    flavor,
  });
  return result.versions;
}

export function resolve(params: ResolveParams): Promise<ServerProfile> {
  return call('server.resolve', params);
}

/** Loader builds for a flavor/version, newest first; empty for vanilla. */
export async function loaders(
  flavor: string,
  version: string,
): Promise<string[]> {
  const result = await call<{ loaders: string[] }>('server.loaders', {
    flavor,
    version,
  });
  return result.loaders;
}

export async function list(): Promise<ServerInfo[]> {
  const result = await call<{ servers: ServerInfo[] }>('server.list');
  return result.servers;
}

/** The server's record merged with its live process state. */
export function status(server: string): Promise<ServerInfo> {
  return call('server.status', { server });
}

/** The server's static, informational view (locations + disk footprint). */
export function info(server: string): Promise<ServerDetails> {
  return call('server.info', { server });
}

/** A Server List Ping snapshot; only a running server answers. */
export function ping(server: string): Promise<ServerPingResult> {
  return call('server.ping', { server });
}

export async function create(
  params: ServerCreateParams,
  onProgress?: OnProgress,
): Promise<ServerInfo> {
  const id = jobId('server-create');
  const done = await runJob<{ id: string; server: ServerInfo }>({
    id,
    topics: {
      progress: 'server.create.progress',
      done: 'server.create.done',
      error: 'server.create.error',
    },
    onProgress,
    start: () => call('server.create', { ...params, id }),
  });
  return done.server;
}

export async function update(
  params: ServerUpdateParams,
  onProgress?: OnProgress,
): Promise<ServerInfo> {
  const id = jobId('server-update');
  const done = await runJob<{ id: string; server: ServerInfo }>({
    id,
    topics: {
      progress: 'server.update.progress',
      done: 'server.update.done',
      error: 'server.update.error',
    },
    onProgress,
    start: () => call('server.update', { ...params, id }),
  });
  return done.server;
}

export function rename(server: string, name: string): Promise<ServerInfo> {
  return call('server.rename', { server, name });
}

export async function remove(server: string): Promise<void> {
  await call('server.remove', { server });
}

export function start(
  server: string,
): Promise<{ processId: string; pid: number }> {
  return call('server.start', { server });
}

export async function stop(server: string): Promise<void> {
  await call('server.stop', { server });
}

export async function logs(
  server: string,
  tail?: number,
): Promise<ProcessLogLine[]> {
  const result = await call<{ lines: ProcessLogLine[] }>('server.logs', {
    server,
    tail,
  });
  return result.lines;
}

/** One console command over the running server's RCON channel. */
export async function command(
  server: string,
  commandLine: string,
): Promise<string> {
  const result = await call<{ response: string }>('server.command', {
    server,
    command: commandLine,
  });
  return result.response;
}

/**
 * The record keys (`memory`, `jvm-args`, `backup-interval`,
 * `backup-retention`) plus any `server.properties` key.
 */
export const config = {
  async get(server: string, key: string): Promise<string | null> {
    const result = await tryCall<{ value: string }>('server.config.get', {
      server,
      key,
    });
    return result?.value ?? null;
  },

  async set(server: string, key: string, value: string): Promise<void> {
    await call('server.config.set', { server, key, value });
  },

  async list(server: string): Promise<ConfigEntry[]> {
    const result = await call<{ entries: ConfigEntry[] }>(
      'server.config.list',
      { server },
    );
    return result.entries;
  },
};

export const backup = {
  /** Archives a running server live (world saving pauses over RCON). */
  async create(server: string, onProgress?: OnProgress): Promise<BackupInfo> {
    const id = jobId('server-backup');
    const done = await runJob<{ id: string; backup: BackupInfo }>({
      id,
      topics: { done: 'backup.done', error: 'backup.error' },
      onProgress,
      start: () => call('server.backup.create', { server, id }),
    });
    return done.backup;
  },

  async list(server: string): Promise<BackupInfo[]> {
    const result = await call<{ backups: BackupInfo[] }>('server.backup.list', {
      server,
    });
    return result.backups;
  },

  /** Refused while the server runs or is busy. */
  async restore(
    server: string,
    backupId: string,
    onProgress?: OnProgress,
  ): Promise<BackupInfo> {
    const id = jobId('server-restore');
    const done = await runJob<{ id: string; backup: BackupInfo }>({
      id,
      topics: { done: 'backup.done', error: 'backup.error' },
      onProgress,
      start: () =>
        call('server.backup.restore', { server, backup: backupId, id }),
    });
    return done.backup;
  },

  async remove(server: string, backupId: string): Promise<void> {
    await call('server.backup.remove', { server, backup: backupId });
  },
};

export const content = {
  /** Servers take mods and datapacks; refused on a running or busy server. */
  add(
    server: string,
    spec: ContentAddSpec,
    onProgress?: OnProgress,
  ): Promise<ContentDone> {
    const id = jobId('server-content');
    return runJob<ContentDone>({
      id,
      topics: {
        progress: 'content.progress',
        done: 'content.done',
        error: 'content.error',
      },
      onProgress,
      start: () => call('server.content.add', { server, ...spec, id }),
    });
  },

  list(server: string, kind: ContentKind): Promise<ContentList> {
    return call('server.content.list', { server, kind });
  },

  async remove(
    server: string,
    kind: ContentKind,
    item: string,
    worlds: string[] = [],
  ): Promise<void> {
    await call('server.content.remove', { server, kind, item, worlds });
  },

  /** `item` empty updates every platform-sourced item of the kind. */
  update(
    server: string,
    kind: ContentKind,
    item = '',
    onProgress?: OnProgress,
  ): Promise<ContentDone> {
    const id = jobId('server-content-update');
    return runJob<ContentDone>({
      id,
      topics: {
        progress: 'content.progress',
        done: 'content.done',
        error: 'content.error',
      },
      onProgress,
      start: () => call('server.content.update', { server, kind, item, id }),
    });
  },

  /** Enable or disable one installed item; applies at the next start. */
  async enable(
    server: string,
    kind: ContentKind,
    item: string,
    enabled: boolean,
    worlds: string[] = [],
  ): Promise<void> {
    await call('server.content.enable', {
      server,
      kind,
      item,
      enabled,
      worlds,
    });
  },

  /** Which platform items of the kind have a newer compatible version. */
  async checkUpdates(
    server: string,
    kind: ContentKind,
  ): Promise<ContentUpdate[]> {
    const result = await call<{ updates: ContentUpdate[] }>(
      'server.content.check_updates',
      { server, kind },
      { timeoutMs: 120_000 },
    );
    return result.updates;
  },

  /** Re-pin one item to a specific published version (id or number). */
  setVersion(
    server: string,
    kind: ContentKind,
    item: string,
    version: string,
    onProgress?: OnProgress,
  ): Promise<ContentDone> {
    const id = jobId('server-content-set-version');
    return runJob<ContentDone>({
      id,
      topics: {
        progress: 'content.progress',
        done: 'content.done',
        error: 'content.error',
      },
      onProgress,
      start: () =>
        call('server.content.set_version', { server, kind, item, version, id }),
    });
  },
};
