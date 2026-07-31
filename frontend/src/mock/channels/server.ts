/**
 * `server.*`. Unlike an instance, a server is fully provisioned at create —
 * so `create` and `update` are jobs, and everything after them reads a record
 * that is already `ready`. Backups live in ./backup; settings and content come
 * from ./entry.
 */
import type { ServerDetails, ServerInfo, ServerProfile } from '@/api/types';

import { jobIdOf, startJob } from '../job';
import * as catalog from '../state/catalog';
import * as entries from '../state/entries';
import * as processes from '../state/processes';
import { type Handlers, num, ok, str } from '../support';
import { channels as backupChannels } from './backup';
import { configChannels, contentChannels } from './entry';

const BANNER = [
  'Starting minecraft server version 1.21.4',
  'Loading properties',
  'Preparing level "world"',
  'Done (4.812s)! For help, type "help"',
];

/** A server record composed with its live process. */
const withProcess = (server: ServerInfo): ServerInfo => ({
  ...server,
  process: processes.serverProcess(server.id),
});

const serverId = (payload: Record<string, unknown>): string =>
  entries.findServer(str(payload, 'server')).id;

export const channels: Handlers = {
  'server.list': () => ({ servers: entries.listServers().map(withProcess) }),

  'server.status': (p) => withProcess(entries.findServer(str(p, 'server'))),

  'server.info': (p): ServerDetails => {
    const server = entries.findServer(str(p, 'server'));
    return {
      ...server,
      ...entries.directories(server.id, 'servers'),
      diskBytes: 1_024 * 1024 * 1024,
      jvmArgs: ['-XX:+UseG1GC'],
      jvmArgsSource: 'defaults',
      warnings: [],
    };
  },

  'server.ping': (p) => {
    const server = entries.findServer(str(p, 'server'));
    return {
      playersOnline: processes.serverProcess(server.id) ? 2 : 0,
      playersMax: 20,
      motd: `${server.name} — a mock server`,
      version: server.gameVersion,
    };
  },

  'server.flavors': () => ({ flavors: catalog.serverFlavors }),
  'server.versions': (p) => ({
    versions: catalog.versionsFor(str(p, 'flavor')),
  }),
  'server.loaders': (p) => ({ loaders: catalog.loaders(str(p, 'flavor')) }),

  'server.resolve': (p): ServerProfile => {
    const version = str(p, 'version', '1.21.4');
    return {
      flavor: str(p, 'flavor', 'vanilla'),
      gameVersion: version,
      loaderVersion: str(p, 'loaderVersion') || undefined,
      primary: {
        url: `https://piston-data.mojang.com/${version}/server.jar`,
        filename: 'server.jar',
        size: 51_000_000,
      },
      libraries: [],
      javaMajor: catalog.javaFor(version),
      mainClass: 'net.minecraft.server.Main',
      jvmArgs: [],
      argsFile: '',
    };
  },

  'server.create': (p) =>
    startJob({
      id: jobIdOf(p, 'server-create'),
      family: 'server.create',
      steps: [
        { phase: 'resolving', detail: str(p, 'version') },
        { phase: 'java', detail: 'Java runtime' },
        { phase: 'server', detail: 'server jar' },
        { phase: 'overrides', detail: 'eula.txt' },
      ],
      done: () => ({
        server: withProcess(
          entries.addServer({
            name: str(p, 'name'),
            flavor: str(p, 'flavor', 'vanilla'),
            version: str(p, 'version'),
            loaderVersion: str(p, 'loaderVersion') || undefined,
            port: num(p, 'port') || undefined,
          }),
        ),
        warnings: [],
      }),
    }),

  'server.update': (p) => {
    const server = entries.findServer(str(p, 'server'));
    return startJob({
      id: jobIdOf(p, 'server-update'),
      family: 'server.update',
      steps: [
        { phase: 'backup', detail: 'backing up first' },
        { phase: 'resolving', detail: str(p, 'version') },
        { phase: 'server', detail: 'server jar' },
      ],
      done: () => {
        server.gameVersion = str(p, 'version', server.gameVersion);
        server.loaderVersion = str(p, 'loaderVersion') || server.loaderVersion;
        server.javaMajor = catalog.javaFor(server.gameVersion);
        return { server: withProcess(server), warnings: [] };
      },
    });
  },

  'server.rename': (p) =>
    withProcess(
      entries.rename(entries.findServer(str(p, 'server')), str(p, 'name')),
    ),

  'server.remove': (p) => {
    entries.removeEntry(serverId(p), entries.listServers());
    return ok();
  },

  'server.start': (p) => {
    const server = entries.findServer(str(p, 'server'));
    const process = processes.start(
      processes.serverProcessId(server.id),
      `${entries.HOME}/java/${server.javaMajor}/bin/java`,
      ['-Xmx4G', '-jar', 'server.jar', 'nogui'],
      BANNER,
    );
    return { processId: process.id, pid: process.pid };
  },

  'server.stop': (p) => {
    processes.stop(processes.serverProcessId(serverId(p)));
    return ok();
  },

  'server.logs': (p) => ({
    lines: processes.logs(
      processes.serverProcessId(serverId(p)),
      p.tail as number | undefined,
    ),
  }),

  // One RCON round trip. Only a running server answers — the console tab
  // renders the rejection, so refusing here is the honest response.
  'server.command': (p) => {
    const id = serverId(p);
    if (!processes.serverProcess(id))
      throw { code: 'handler_error', message: 'the server is not running' };
    const command = str(p, 'command');
    return { response: `Unknown or incomplete command: ${command}` };
  },

  ...configChannels('server', serverId),
  ...contentChannels('server', serverId),
  ...backupChannels(serverId),
};
