// Canned daemon responses for browser dev, one per request channel. Shapes
// mirror what each `src/api` function unwraps; an unlisted channel falls back
// to a never-undefined empty in ./index, so a new channel degrades, not crashes.

type Handler = (payload: Record<string, unknown>) => unknown;

const now = () => Math.floor(Date.now() / 1000);
const ok = () => ({});

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
  sessions: [],
  accepts: ['mod', 'resource_pack', 'shader', 'data_pack'],
};

const server = {
  id: 'b1',
  name: 'SMP',
  flavor: 'vanilla',
  gameVersion: '1.21.1',
  javaMajor: 21,
  createdUnix: now() - 172_800,
  ready: true,
  gamePort: 25_565,
  console: true,
  accepts: ['data_pack'],
};

const instanceDetails = {
  ...instance,
  entryDir: `/mock/.hestia/instances/${instance.id}`,
  dataDir: `/mock/.hestia/instances/${instance.id}/data`,
  diskBytes: 512 * 1024 * 1024,
};

const serverDetails = {
  ...server,
  entryDir: `/mock/.hestia/servers/${server.id}`,
  dataDir: `/mock/.hestia/servers/${server.id}/data`,
  diskBytes: 1_024 * 1024 * 1024,
};

const contentList = () => ({ items: [], untracked: [] });
const configList = () => ({ entries: [] });
const logs = () => ({ lines: [] });

/** Channel → response. Add or correct a line here if a page shows wrong data. */
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
  'daemon.stop': () => ({ stopping: true }),

  'account.list': () => ({ accounts: [account], defaultUuid: account.uuid }),
  'account.login.begin': () => ({
    id: 'login-1',
    method: 'device_code',
    userCode: 'MOCK-CODE',
    verificationUri: 'https://microsoft.com/link',
  }),
  'account.login.complete': () => ({ account }),
  'account.switch': () => ({ account }),
  'account.remove': ok,

  'cache.info': () => ({ entries: 0, bytes: 0, path: '/mock/.hestia/cache' }),
  'cache.list': () => ({ entries: [] }),
  'cache.clear': ok,

  'config.get': () => ({ value: null }),
  'config.list': () => ({ entries: {} }),
  'config.set': ok,

  'content.sources': () => ({
    sources: [{ id: 'modrinth', name: 'Modrinth' }],
  }),
  'content.search': () => ({ hits: [], offset: 0, limit: 20, total: 0 }),
  'content.versions': () => ({ versions: [] }),
  'content.project': (p) => ({
    source: 'modrinth',
    id: String(p.project ?? 'mock'),
    slug: String(p.project ?? 'mock'),
    kind: 'mod',
    title: 'Mock Project',
    description: 'A fixture project.',
    body: '',
    author: 'mock',
    categories: [],
    downloads: 0,
    follows: 0,
    iconUrl: '',
    gallery: [],
    clientSide: 'optional',
    serverSide: 'optional',
  }),
  'content.resolve_url': (p) => {
    const url = String(p.url ?? '');
    const [, type = 'mod', slug = 'mock', version = ''] =
      /\/(mod|modpack|resourcepack|shader|datapack|plugin)\/([^/?#]+)(?:\/version\/([^/?#]+))?/.exec(
        url,
      ) ?? [];
    const kinds: Record<string, string> = {
      mod: 'mod',
      modpack: 'modpack',
      resourcepack: 'resource_pack',
      shader: 'shader',
      datapack: 'data_pack',
      plugin: 'plugin',
    };
    return {
      project: {
        source: 'modrinth',
        id: slug,
        slug,
        kind: kinds[type] ?? 'mod',
        kinds: [],
        title: `Mock ${slug}`,
        description: 'Resolved from a pasted link.',
        body: '',
        author: 'mock',
        categories: [],
        downloads: 0,
        follows: 0,
        iconUrl: '',
        gallery: [],
        clientSide: 'optional',
        serverSide: 'optional',
      },
      versionId: version,
    };
  },
  'content.inspect': (p) => ({
    valid: true,
    kind: 'mod',
    filename:
      String(p.path ?? '')
        .split(/[/\\]/)
        .pop() ?? '',
    reason: '',
  }),
  'content.modpack.resolve': () => ({ files: [], loader: null }),

  'instance.list': () => ({ instances: [instance] }),
  'instance.info': () => instanceDetails,
  'instance.flavors': () => ({ flavors }),
  'instance.loaders': () => ({ loaders: ['0.16.5'] }),
  'instance.versions': () => ({ versions }),
  'instance.worlds': () => ({ worlds: [] }),
  'instance.logs': logs,
  'instance.resolve': () => ({}),
  'instance.create': () => ({ instance }),
  'instance.update': () => ({ instance }),
  'instance.launch': ok,
  'instance.stop': ok,
  'instance.remove': ok,
  'instance.rename': ok,
  'instance.config.get': () => ({ value: '' }),
  'instance.config.list': configList,
  'instance.config.set': ok,
  'instance.content.list': contentList,
  'instance.content.add': ok,
  'instance.content.enable': ok,
  'instance.content.remove': ok,
  'instance.content.update': ok,
  'instance.profile.list': () => ({ active: '', profiles: [] }),
  'instance.profile.apply': ok,
  'instance.profile.capture': ok,
  'instance.profile.create': ok,
  'instance.profile.edit': ok,
  'instance.profile.release': ok,
  'instance.profile.remove': ok,
  'instance.profile.rename': ok,
  'instance.profile.use': ok,
  'instance.sync.adopt': () => ({ adopted: [] }),

  'sync.get': () => ({
    sharedDir: '/mock/.hestia/shared',
    targets: { files: [], folders: [] },
  }),
  'sync.set': () => ({
    sharedDir: '/mock/.hestia/shared',
    targets: { files: [], folders: [] },
  }),
  'sync.status': () => ({ instances: [] }),

  'server.list': () => ({ servers: [server] }),
  'server.info': () => serverDetails,
  'server.status': () => server,
  'server.ping': () => ({
    playersOnline: 0,
    playersMax: 20,
    motd: 'A Mock Server',
    version: '1.21.1',
  }),
  'server.flavors': () => ({ flavors }),
  'server.loaders': () => ({ loaders: [] }),
  'server.versions': () => ({ versions }),
  'server.logs': logs,
  'server.resolve': () => ({}),
  'server.create': ok,
  'server.update': ok,
  'server.start': ok,
  'server.stop': ok,
  'server.remove': ok,
  'server.rename': ok,
  'server.command': () => ({ response: '' }),
  'server.config.get': () => ({ value: '' }),
  'server.config.list': configList,
  'server.config.set': ok,
  'server.content.list': contentList,
  'server.content.add': ok,
  'server.content.enable': ok,
  'server.content.remove': ok,
  'server.content.update': ok,
  'server.backup.list': () => ({ backups: [] }),
  'server.backup.create': ok,
  'server.backup.remove': ok,
  'server.backup.restore': ok,

  'process.list': () => ({ processes: [] }),
  'process.status': (p) => ({
    id: String(p.id ?? 'p1'),
    pid: 0,
    program: '',
    args: [],
    state: 'exited',
    startedUnix: now(),
  }),
  'process.logs': logs,
  'process.start': ok,
  'process.stop': ok,

  'java.list': () => ({ runtimes: [] }),
  'java.releases': () => ({ releases: [] }),
  'java.install': ok,
  'java.uninstall': ok,
  'download.start': ok,

  'skin.list': () => ({ skins: [], capes: [] }),
  'skin.add': () => ({ skin: null }),
  'skin.update': () => ({ skin: null }),
  'skin.equip': ok,
  'skin.reset': ok,
  'skin.remove': ok,
  'cape.equip': ok,
  'cape.clear': ok,

  'profile.list': () => ({ profiles: [] }),
  'profile.create': ok,
  'profile.edit': ok,
  'profile.remove': ok,
};

/** Bespoke shell commands (not the generic `ipc_call` bridge). */
export const commands: Record<string, Handler> = {
  prefs_list: () => ({}),
  prefs_set: () => null,
  prefs_remove: () => null,
  icons_list: () => [],
  icon_set: () => null,
  icon_remove: () => null,
  log_write: () => null,
  crash_report: () => null,
  crash_list: () => [],
  crash_read: () => '',
  crash_clear: () => null,
  start_daemon: () => channels['daemon.status']({}),
  'plugin:opener|open_url': (args) => {
    const url = args.url;
    if (typeof url === 'string') window.open(url, '_blank', 'noopener');
    return null;
  },
  'plugin:opener|open_path': () => null,
};
