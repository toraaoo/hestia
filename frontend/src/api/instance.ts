/**
 * The `instance.*` channels. Unlike a server, an instance is a lightweight
 * record at create — its files materialise during the `launch` job. An
 * instance can run several concurrent sessions (opt-in via `new_session`);
 * `stop` and `logs` take an optional session id.
 */

import { call, tryCall } from './core/ipc';
import { jobId, runJob } from './core/jobs';
import type {
  ContentAddSpec,
  ContentDone,
  ContentKind,
  ContentList,
} from './types/content';
import type {
  ContentProfile,
  InstanceCreateParams,
  InstanceInfo,
  InstanceLaunchDone,
  InstanceLaunchParams,
  InstanceProfiles,
  InstanceUpdateParams,
} from './types/instance';
import type {
  ConfigEntry,
  Flavor,
  GameVersion,
  InstanceProfile,
  ProvisionProgress,
  ResolveParams,
} from './types/minecraft';
import type { ProcessLogLine } from './types/process';

type OnProgress = (progress: ProvisionProgress) => void;

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

export async function list(): Promise<InstanceInfo[]> {
  const result = await call<{ instances: InstanceInfo[] }>('instance.list');
  return result.instances;
}

export async function create(
  params: InstanceCreateParams,
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
  params: InstanceUpdateParams,
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

/** Save-world folder names, for the datapack world picker. */
export async function worlds(instance: string): Promise<string[]> {
  const result = await call<{ worlds: string[] }>('instance.worlds', {
    instance,
  });
  return result.worlds;
}

/**
 * Materialise the instance's files and spawn the game as the signed-in
 * account. Resolves once the session is running.
 */
export function launch(
  params: InstanceLaunchParams,
  onProgress?: OnProgress,
): Promise<InstanceLaunchDone> {
  const id = jobId('instance-launch');
  return runJob<InstanceLaunchDone>({
    id,
    topics: {
      progress: 'instance.launch.progress',
      done: 'instance.launch.done',
      error: 'instance.launch.error',
    },
    onProgress,
    start: () => call('instance.launch', { ...params, id }),
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
  list(instance: string): Promise<InstanceProfiles> {
    return call('instance.profile.list', { instance });
  },

  /** Seeded with every selectable pool item unless `seedFromPool` is false. */
  create(
    instance: string,
    name: string,
    seedFromPool = true,
  ): Promise<ContentProfile> {
    return call('instance.profile.create', {
      instance,
      name,
      seed_from_pool: seedFromPool,
    });
  },

  /** Removing the active profile clears the active selection. */
  async remove(instance: string, name: string): Promise<void> {
    await call('instance.profile.remove', { instance, name });
  },

  rename(
    instance: string,
    name: string,
    newName: string,
  ): Promise<ContentProfile> {
    return call('instance.profile.rename', {
      instance,
      name,
      new_name: newName,
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
  ): Promise<ContentProfile> {
    return call('instance.profile.edit', { instance, name, add, remove });
  },
};

export const content = {
  /** Instances take mods, resourcepacks, shaders, and datapacks. */
  add(
    instance: string,
    spec: ContentAddSpec,
    onProgress?: OnProgress,
  ): Promise<ContentDone> {
    const id = jobId('instance-content');
    return runJob<ContentDone>({
      id,
      topics: { done: 'content.done', error: 'content.error' },
      onProgress,
      start: () => call('instance.content.add', { instance, ...spec, id }),
    });
  },

  list(instance: string, kind: ContentKind): Promise<ContentList> {
    return call('instance.content.list', { instance, kind });
  },

  async remove(
    instance: string,
    kind: ContentKind,
    item: string,
    worlds: string[] = [],
  ): Promise<void> {
    await call('instance.content.remove', { instance, kind, item, worlds });
  },

  /** `item` empty updates every platform-sourced item of the kind. */
  update(
    instance: string,
    kind: ContentKind,
    item = '',
    onProgress?: OnProgress,
  ): Promise<ContentDone> {
    const id = jobId('instance-content-update');
    return runJob<ContentDone>({
      id,
      topics: { done: 'content.done', error: 'content.error' },
      onProgress,
      start: () =>
        call('instance.content.update', { instance, kind, item, id }),
    });
  },
};
