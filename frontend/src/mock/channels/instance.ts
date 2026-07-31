/**
 * `instance.*`. An instance is a lightweight record at create; its files
 * materialise during the `launch` job, which here means a supervised session
 * appearing and starting to broadcast. Content and settings come from ./entry,
 * which a server shares.
 */
import type {
  InstanceDetails,
  InstanceInfo,
  InstanceProfile,
  Profile,
  ServerEntry,
} from '@/api/types';

import { jobIdOf, startJob } from '../job';
import * as catalog from '../state/catalog';
import * as content from '../state/content';
import * as entries from '../state/entries';
import * as processes from '../state/processes';
import * as worlds from '../state/worlds';
import { bool, fail, type Handlers, now, ok, str, strings } from '../support';
import { resolve as resolveAccount } from './account';
import { configChannels, contentChannels } from './entry';

const BANNER = [
  '[main/INFO]: Setting user: Player',
  '[main/INFO]: Backend library: LWJGL version 3.3.3',
  '[Render thread/INFO]: OpenAL initialized on device Mock Output',
];

/** An instance record composed with its live sessions. */
const withSessions = (instance: InstanceInfo): InstanceInfo => ({
  ...instance,
  sessions: processes.sessionsOf(instance.id),
});

const instanceId = (payload: Record<string, unknown>): string =>
  entries.findInstance(str(payload, 'instance')).id;

/** Per-instance content profiles: named selections over the installed pool. */
const profiles = new Map<string, { active: string; profiles: Profile[] }>();

const profilesOf = (id: string) => {
  const existing = profiles.get(id);
  if (existing) return existing;
  const fresh = { active: '', profiles: [] as Profile[] };
  profiles.set(id, fresh);
  return fresh;
};

function findProfile(id: string, name: string): Profile {
  const found = profilesOf(id).profiles.find(
    (profile) => profile.name === name,
  );
  if (!found) fail('not_found', `no such profile: ${name}`);
  return found;
}

export const channels: Handlers = {
  'instance.list': () => ({
    instances: entries.listInstances().map(withSessions),
  }),

  'instance.info': (p): InstanceDetails => {
    const instance = entries.findInstance(str(p, 'instance'));
    return {
      ...instance,
      ...entries.directories(instance.id, 'instances'),
      diskBytes: 512 * 1024 * 1024,
    };
  },

  'instance.flavors': () => ({ flavors: catalog.instanceFlavors }),
  'instance.versions': (p) => ({
    versions: catalog.versionsFor(str(p, 'flavor')),
  }),
  'instance.loaders': (p) => ({ loaders: catalog.loaders(str(p, 'flavor')) }),

  'instance.resolve': (p): InstanceProfile => {
    const version = str(p, 'version', '1.21.4');
    return {
      flavor: str(p, 'flavor', 'vanilla'),
      gameVersion: version,
      loaderVersion: str(p, 'loaderVersion') || undefined,
      client: {
        url: `https://piston-data.mojang.com/${version}/client.jar`,
        filename: `${version}.jar`,
        size: 26_000_000,
      },
      libraries: [],
      assetIndex: {
        id: version,
        artifact: {
          url: `https://piston-meta.mojang.com/${version}.json`,
          filename: `${version}.json`,
          size: 420_000,
        },
        totalSize: 780_000_000,
      },
      javaMajor: catalog.javaFor(version),
      mainClass: 'net.minecraft.client.main.Main',
      jvmArgs: [],
      gameArgs: [],
    };
  },

  'instance.create': (p) => ({
    instance: withSessions(
      entries.addInstance({
        name: str(p, 'name'),
        flavor: str(p, 'flavor', 'vanilla'),
        version: str(p, 'version'),
        loaderVersion: str(p, 'loaderVersion') || undefined,
      }),
    ),
  }),

  'instance.update': (p) => {
    const instance = entries.findInstance(str(p, 'instance'));
    instance.gameVersion = str(p, 'version', instance.gameVersion);
    instance.loaderVersion = str(p, 'loaderVersion') || instance.loaderVersion;
    instance.javaMajor = catalog.javaFor(instance.gameVersion);
    return { instance: withSessions(instance) };
  },

  'instance.rename': (p) =>
    withSessions(
      entries.rename(entries.findInstance(str(p, 'instance')), str(p, 'name')),
    ),

  'instance.remove': (p) => {
    entries.removeEntry(instanceId(p), entries.listInstances());
    return ok();
  },

  'instance.worlds': (p) => ({ worlds: worlds.worldsOf(instanceId(p)) }),

  'instance.servers': (p) => ({ servers: worlds.serversOf(instanceId(p)) }),

  'instance.server.edit': (p) => {
    const id = instanceId(p);
    const entry: ServerEntry = {
      name: str(p, 'name'),
      address: str(p, 'address'),
      icon: '',
      acceptTextures: bool(p, 'acceptTextures'),
      hidden: false,
    };
    return {
      servers: worlds.editServer(id, str(p, 'server'), entry),
      warnings: [],
    };
  },

  'instance.server.remove': (p) => ({
    servers: worlds.removeServer(instanceId(p), str(p, 'server')),
    warnings: [],
  }),

  'minecraft.ping': (p) => ({
    playersOnline: 3,
    playersMax: 20,
    motd: `A Mock Server — ${str(p, 'address', 'localhost')}`,
    version: '1.21.4',
  }),

  'instance.launch': (p) => {
    const instance = entries.findInstance(str(p, 'instance'));
    const account = resolveAccount(str(p, 'account'));
    return startJob({
      id: jobIdOf(p, 'instance-launch'),
      family: 'instance.launch',
      steps: [
        { phase: 'resolving', detail: instance.gameVersion },
        { phase: 'java', detail: `Java ${instance.javaMajor}` },
        { phase: 'libraries', detail: 'libraries' },
        { phase: 'assets', detail: 'assets' },
        { phase: 'client', detail: 'client jar' },
      ],
      done: () => {
        const id = processes.sessionId(instance.id);
        const session = processes.start(
          id,
          `${entries.HOME}/java/21/bin/java`,
          ['-Xmx6G', 'net.minecraft.client.main.Main'],
          [`[main/INFO]: Setting user: ${account.name}`, ...BANNER.slice(1)],
        );
        instance.lastPlayedUnix = now();
        return { processId: session.id, pid: session.pid, warnings: [] };
      },
    });
  },

  'instance.stop': (p) => {
    processes.stopSessions(instanceId(p), str(p, 'session') || undefined);
    return ok();
  },

  'instance.logs': (p) => {
    const id = instanceId(p);
    const session =
      str(p, 'session') || processes.sessionsOf(id).at(-1)?.id || '';
    return { lines: processes.logs(session, p.tail as number | undefined) };
  },

  'instance.profile.list': (p) => profilesOf(instanceId(p)),

  'instance.profile.create': (p) => {
    const id = instanceId(p);
    const profile: Profile = {
      name: str(p, 'name'),
      members: bool(p, 'seedFromPool', true)
        ? content.poolOf(id).map((item) => item.filename)
        : [],
      captured: false,
    };
    profilesOf(id).profiles.push(profile);
    return profile;
  },

  'instance.profile.remove': (p) => {
    const id = instanceId(p);
    const store = profilesOf(id);
    const name = str(p, 'name');
    store.profiles = store.profiles.filter((profile) => profile.name !== name);
    if (store.active === name) store.active = '';
    return ok();
  },

  'instance.profile.rename': (p) => {
    const profile = findProfile(instanceId(p), str(p, 'name'));
    profile.name = str(p, 'newName');
    return profile;
  },

  'instance.profile.use': (p) => {
    profilesOf(instanceId(p)).active = str(p, 'name');
    return ok();
  },

  'instance.profile.edit': (p) => {
    const profile = findProfile(instanceId(p), str(p, 'name'));
    const removed = new Set(strings(p, 'remove'));
    profile.members = [
      ...profile.members.filter((member) => !removed.has(member)),
      ...strings(p, 'add'),
    ];
    return profile;
  },

  'instance.profile.capture': (p) => {
    findProfile(instanceId(p), str(p, 'name')).captured = true;
    return ok();
  },

  'instance.profile.release': (p) => {
    findProfile(instanceId(p), str(p, 'name')).captured = false;
    return ok();
  },

  // Applying a *global* profile is a content job: its references install into
  // the pool tagged `profile:<name>`, and nothing is ever removed by it.
  'instance.profile.apply': (p) => {
    const id = instanceId(p);
    const name = str(p, 'profile');
    return startJob({
      id: jobIdOf(p, 'profile-apply'),
      family: 'content',
      steps: [
        { phase: 'resolving', detail: name },
        { phase: 'content', detail: 'installing' },
      ],
      done: () => ({
        items: content.install(id, 'mod', ['lithium'], `profile:${name}`),
        failures: [],
      }),
    });
  },

  ...configChannels('instance', instanceId),
  ...contentChannels('instance', instanceId),
};
