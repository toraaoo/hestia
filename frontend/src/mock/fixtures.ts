// Canned daemon responses for browser dev. A missing channel falls through to
// `{}` with a console warning, so a blank page points at the channel to add.

type Handler = (payload: Record<string, unknown>) => unknown;

const now = () => Math.floor(Date.now() / 1000);

const flavors = [
  { id: 'vanilla', name: 'Vanilla' },
  { id: 'fabric', name: 'Fabric' },
];

const versions = [
  { id: '1.21.1', kind: 'release', stable: true },
  { id: '1.21', kind: 'release', stable: true },
  { id: '24w14a', kind: 'snapshot', stable: false },
];

const account = {
  uuid: '00000000-0000-0000-0000-000000000001',
  name: 'DevPlayer',
  needsReauth: false,
};

const instance = {
  id: 'a1',
  name: 'Fabric Playground',
  flavor: 'fabric',
  gameVersion: '1.21.1',
  loaderVersion: '0.16.5',
  javaMajor: 21,
  createdUnix: now() - 86_400,
  lastPlayedUnix: now() - 3_600,
  playtimeSeconds: 7_200,
};

const server = {
  id: 'b1',
  name: 'SMP',
  flavor: 'vanilla',
  gameVersion: '1.21.1',
  javaMajor: 21,
  createdUnix: now() - 172_800,
  ready: true,
  gamePort: 25565,
  console: true,
};

/** Channel → response. Add a line here when a page shows an empty state. */
export const channels: Record<string, Handler> = {
  'health.ping': () => ({ status: 'ok', pid: 4242 }),
  'app.info': () => ({
    name: 'Hestia',
    version: '0.0.1-mock',
    id: 'gg.toraaoo.hestia',
    vendor: 'toraaoo',
    channel: 'dev',
  }),
  'daemon.status': () => ({
    pid: 4242,
    version: '0.0.1-mock',
    uptimeSeconds: 1_337,
    home: '/mock/.hestia',
    log: '/mock/.hestia/logs/latest.log',
  }),

  'account.list': () => ({ accounts: [account], defaultUuid: account.uuid }),

  'instance.list': () => ({ instances: [instance] }),
  'instance.info': () => ({
    ...instance,
    entryDir: `/mock/.hestia/instances/${instance.id}`,
    dataDir: `/mock/.hestia/instances/${instance.id}/data`,
    diskBytes: 512 * 1024 * 1024,
  }),
  'instance.content.list': () => ({ content: [] }),
  'instance.profile.list': () => ({ profiles: [], active: null }),
  'instance.worlds': () => ({ worlds: [] }),
  'instance.flavors': () => ({ flavors }),
  'instance.loaders': () => ({ loaders: ['0.16.5'] }),
  'instance.versions': () => ({ versions }),

  'server.list': () => ({ servers: [server] }),
  'server.info': () => ({
    ...server,
    entryDir: `/mock/.hestia/servers/${server.id}`,
    dataDir: `/mock/.hestia/servers/${server.id}/data`,
    diskBytes: 1_024 * 1024 * 1024,
  }),
  'server.content.list': () => ({ content: [] }),
  'server.backup.list': () => ({ backups: [] }),
  'server.flavors': () => ({ flavors }),
  'server.loaders': () => ({ loaders: [] }),
  'server.versions': () => ({ versions }),

  'process.list': () => ({ processes: [] }),
  'java.list': () => ({ runtimes: [] }),
  'cache.info': () => ({ count: 0, bytes: 0 }),
  'cache.list': () => ({ entries: [] }),
  'content.sources': () => ({
    sources: [{ id: 'modrinth', name: 'Modrinth' }],
  }),
  'profile.list': () => ({ profiles: [] }),
  'config.list': () => ({ entries: [] }),
  'config.get': () => ({ value: null }),
};

/** Bespoke shell commands (not the generic `ipc_call` bridge). */
export const commands: Record<string, Handler> = {
  prefs_list: () => ({}),
  prefs_set: () => null,
  prefs_remove: () => null,
  icons_list: () => [],
  icon_set: () => null,
  icon_remove: () => null,
  start_daemon: () => channels['daemon.status']({}),
  'plugin:opener|open_url': (args) => {
    const url = args.url;
    if (typeof url === 'string') window.open(url, '_blank', 'noopener');
    return null;
  },
  'plugin:opener|open_path': () => null,
};
