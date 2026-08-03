/**
 * The `instance.*` channels. Unlike a server, an instance is a lightweight
 * record at create — its files materialise during the `launch` job. An
 * instance can run several concurrent sessions (opt-in via `new_session`);
 * `stop` and `logs` take an optional session id.
 */

import type { ContentAddInput } from './content';
import { call, tryCall } from './core/ipc';
import { type JobRun, runJob } from './core/jobs';
import type {
  ContentDoneEvent,
  ContentKind,
  ContentListResult,
  ContentUpdate,
} from './types/content';
import type {
  InstanceCreateParams,
  InstanceDetails,
  InstanceInfo,
  InstanceLaunchDoneEvent,
  InstanceLaunchParams,
  InstanceProfileListResult,
  InstanceServerEditParams,
  InstanceServersWriteResult,
  InstanceUpdateParams,
  Profile,
  ServerEntry,
  WorldInfo,
} from './types/instance';
import type {
  ConfigEntry,
  Flavor,
  GameVersion,
  InstanceProfile,
  ResolveParams,
} from './types/minecraft';
import type { ProcessLogLine } from './types/process';
import type { ServerPingResult } from './types/server';

export async function flavors(): Promise<Flavor[]> {
  const result = await call<{ flavors: Flavor[] }>('instance.flavors');
  return result.flavors;
}

export async function versions(flavor: string): Promise<GameVersion[]> {
  const result = await call<{ versions: GameVersion[] }>('instance.versions', {
    flavor,
  });
  return result.versions;
}

export function resolve(params: ResolveParams): Promise<InstanceProfile> {
  return call('instance.resolve', params);
}

/** Loader builds for a flavor/version, newest first; empty for vanilla. */
export async function loaders(
  flavor: string,
  version: string,
): Promise<string[]> {
  const result = await call<{ loaders: string[] }>('instance.loaders', {
    flavor,
    version,
  });
  return result.loaders;
}

export async function list(): Promise<InstanceInfo[]> {
  const result = await call<{ instances: InstanceInfo[] }>('instance.list');
  return result?.instances ?? [];
}

/** The instance's static, informational view (locations + disk footprint). */
export function info(instance: string): Promise<InstanceDetails> {
  return call('instance.info', { instance });
}

export async function create(
  params: Partial<InstanceCreateParams>,
): Promise<InstanceInfo> {
  const result = await call<{ instance: InstanceInfo }>(
    'instance.create',
    params,
    { timeoutMs: 60_000 },
  );
  return result.instance;
}

/** The instance pays for the new version at its next launch. */
export async function update(
  params: Omit<InstanceUpdateParams, 'id'>,
): Promise<InstanceInfo> {
  const result = await call<{ instance: InstanceInfo }>(
    'instance.update',
    params,
    { timeoutMs: 10 * 60_000 },
  );
  return result.instance;
}

export function rename(instance: string, name: string): Promise<InstanceInfo> {
  return call('instance.rename', { instance, name });
}

export async function remove(instance: string): Promise<void> {
  await call('instance.remove', { instance });
}

/** The save worlds, each described from its own `level.dat`. */
export async function worlds(instance: string): Promise<WorldInfo[]> {
  const result = await call<{ worlds: WorldInfo[] }>('instance.worlds', {
    instance,
  });
  return result.worlds;
}

/** The multiplayer list (`servers.dat`), in the order the game shows it. */
export async function servers(instance: string): Promise<ServerEntry[]> {
  const result = await call<{ servers: ServerEntry[] }>('instance.servers', {
    instance,
  });
  return result.servers;
}

/**
 * Add an entry to the multiplayer list, or rewrite the one `server` names
 * (empty adds). The running game owns the file, so the result carries a
 * warning when a session is open to say the edit will be overwritten.
 */
export function serverEdit(
  params: Partial<InstanceServerEditParams>,
): Promise<InstanceServersWriteResult> {
  return call('instance.server.edit', params);
}

export function serverRemove(
  instance: string,
  server: string,
): Promise<InstanceServersWriteResult> {
  return call('instance.server.remove', { instance, server });
}

/**
 * Move an entry within the multiplayer list. `position` counts over the
 * visible entries — the rows that were listed — from zero.
 */
export function serverMove(
  instance: string,
  server: string,
  position: number,
): Promise<InstanceServersWriteResult> {
  return call('instance.server.move', { instance, server, position });
}

/** Status of a multiplayer address, over the Server List Ping. */
export function pingAddress(address: string): Promise<ServerPingResult> {
  return call('minecraft.ping', { address });
}

/**
 * Materialise the instance's files and spawn the game as the signed-in
 * account. Resolves once the session is running.
 */
export function launch(
  params: Partial<InstanceLaunchParams>,
  job: JobRun,
): Promise<InstanceLaunchDoneEvent> {
  return runJob<InstanceLaunchDoneEvent>({
    ...job,
    topics: {
      progress: 'instance.launch.progress',
      done: 'instance.launch.done',
      error: 'instance.launch.error',
    },
    start: () => call('instance.launch', { ...params, id: job.id }),
  });
}

/** Stops one named session, or every session of the instance. */
export async function stop(instance: string, session?: string): Promise<void> {
  await call('instance.stop', { instance, session });
}

/** Targets the newest running session unless one is named. */
export async function logs(
  instance: string,
  options: { session?: string; tail?: number } = {},
): Promise<ProcessLogLine[]> {
  const result = await call<{ lines: ProcessLogLine[] }>('instance.logs', {
    instance,
    session: options.session,
    tail: options.tail,
  });
  return result.lines;
}

/** `memory` and `jvm-args` only. */
export const config = {
  async get(instance: string, key: string): Promise<string | null> {
    const result = await tryCall<{ value: string }>('instance.config.get', {
      instance,
      key,
    });
    return result?.value ?? null;
  },

  async set(instance: string, key: string, value: string): Promise<void> {
    await call('instance.config.set', { instance, key, value });
  },

  async list(instance: string): Promise<ConfigEntry[]> {
    const result = await call<{ entries: ConfigEntry[] }>(
      'instance.config.list',
      { instance },
    );
    return result.entries;
  },
};

/**
 * Per-instance content profiles: named selections over the installed pool,
 * enforced by the launch-time mirror reconcile. CRUD applies at the next
 * launch, so it is safe while the instance runs.
 */
export const profiles = {
  list(instance: string): Promise<InstanceProfileListResult> {
    return call('instance.profile.list', { instance });
  },

  /** Seeded with every selectable pool item unless `seedFromPool` is false. */
  create(
    instance: string,
    name: string,
    seedFromPool = true,
  ): Promise<Profile> {
    return call('instance.profile.create', {
      instance,
      name,
      seedFromPool,
    });
  },

  /** Removing the active profile clears the active selection. */
  async remove(instance: string, name: string): Promise<void> {
    await call('instance.profile.remove', { instance, name });
  },

  rename(instance: string, name: string, newName: string): Promise<Profile> {
    return call('instance.profile.rename', {
      instance,
      name,
      newName,
    });
  },

  /** Sets the active profile; an empty `name` clears it. */
  async use(instance: string, name: string): Promise<void> {
    await call('instance.profile.use', { instance, name });
  },

  /**
   * Add/remove members by pool reference (project id, slug, filename, or
   * title); a reference that matches nothing — or only a datapack — errors.
   */
  edit(
    instance: string,
    name: string,
    add: string[] = [],
    remove: string[] = [],
  ): Promise<Profile> {
    return call('instance.profile.edit', { instance, name, add, remove });
  },

  /**
   * Capture the profile's own settings store (snapshotted from the global
   * one); launches under it then sync settings against the captured store.
   * The instance must be stopped.
   */
  async capture(instance: string, name: string): Promise<void> {
    await call('instance.profile.capture', { instance, name });
  },

  /**
   * Delete the profile's captured store; it inherits the global store again.
   * The instance must be stopped.
   */
  async release(instance: string, name: string): Promise<void> {
    await call('instance.profile.release', { instance, name });
  },

  /**
   * Apply a **global** profile into the instance's pool — a content job:
   * references not already present install at their newest compatible
   * version, tagged `profile:<name>`; incompatible ones come back as
   * failures. Applying never removes de-listed content. Refused on a running
   * or busy instance.
   */
  apply(
    instance: string,
    profile: string,
    job: JobRun,
  ): Promise<ContentDoneEvent> {
    return runJob<ContentDoneEvent>({
      ...job,
      topics: { done: 'content.done', error: 'content.error' },
      start: () =>
        call('instance.profile.apply', { instance, profile, id: job.id }),
    });
  },
};

export const content = {
  /** Instances take mods, resourcepacks, shaders, and datapacks. */
  add(
    instance: string,
    spec: ContentAddInput,
    job: JobRun,
  ): Promise<ContentDoneEvent> {
    return runJob<ContentDoneEvent>({
      ...job,
      topics: {
        progress: 'content.progress',
        done: 'content.done',
        error: 'content.error',
      },
      start: () =>
        call('instance.content.add', { instance, ...spec, id: job.id }),
    });
  },

  list(instance: string, kind: ContentKind): Promise<ContentListResult> {
    return call('instance.content.list', { instance, kind });
  },

  /** Uninstalls the named items in one call; a name matching nothing takes none. */
  async remove(
    instance: string,
    kind: ContentKind,
    items: string[],
    worlds: string[] = [],
  ): Promise<void> {
    await call('instance.content.remove', { instance, kind, items, worlds });
  },

  /** `items` empty updates every platform-sourced item of the kind. */
  update(
    instance: string,
    kind: ContentKind,
    items: string[],
    job: JobRun,
  ): Promise<ContentDoneEvent> {
    return runJob<ContentDoneEvent>({
      ...job,
      topics: {
        progress: 'content.progress',
        done: 'content.done',
        error: 'content.error',
      },
      start: () =>
        call('instance.content.update', { instance, kind, items, id: job.id }),
    });
  },

  /** Enable or disable one installed item; applies at the next launch. */
  async enable(
    instance: string,
    kind: ContentKind,
    item: string,
    enabled: boolean,
    worlds: string[] = [],
  ): Promise<void> {
    await call('instance.content.enable', {
      instance,
      kind,
      item,
      enabled,
      worlds,
    });
  },

  /** Which platform items of the kind have a newer compatible version. */
  async checkUpdates(
    instance: string,
    kind: ContentKind,
  ): Promise<ContentUpdate[]> {
    const result = await call<{ updates: ContentUpdate[] }>(
      'instance.content.check_updates',
      { instance, kind },
      { timeoutMs: 120_000 },
    );
    return result.updates;
  },

  /** Re-pin one item to a specific published version (id or number). */
  setVersion(
    instance: string,
    kind: ContentKind,
    item: string,
    version: string,
    job: JobRun,
  ): Promise<ContentDoneEvent> {
    return runJob<ContentDoneEvent>({
      ...job,
      topics: {
        progress: 'content.progress',
        done: 'content.done',
        error: 'content.error',
      },
      start: () =>
        call('instance.content.set_version', {
          instance,
          kind,
          item,
          version,
          id: job.id,
        }),
    });
  },
};
