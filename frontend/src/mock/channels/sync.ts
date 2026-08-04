/**
 * `sync.*` — the shared settings/config target set: files copied into the
 * shared store, folders linked into it. `sync.enabled` is a config key, so the
 * switch is read from the settings store rather than held twice.
 */
import type { InstanceSyncStatus, SyncConfig, SyncTargets } from '@/api/types';

import * as entries from '../state/entries';
import * as settings from '../state/settings';
import { type Handlers, str, strings } from '../support';

let targets: SyncTargets = {
  files: ['options.txt', 'servers.dat'],
  folders: ['saves', 'screenshots'],
};

const config = (): SyncConfig => ({
  enabled: settings.enabled('sync.enabled'),
  sharedDir: `${entries.HOME}/shared`,
  targets,
});

/** Instances that opted out, by id. */
const opted_out = new Set<string>();

const status = (): InstanceSyncStatus[] =>
  entries.listInstances().map((instance, index) => ({
    id: instance.id,
    name: instance.name,
    enabled: !opted_out.has(instance.id),
    targets: opted_out.has(instance.id)
      ? []
      : targets.folders.map((target, position) => ({
          target,
          state:
            index === 0 || position === 0
              ? 'linked'
              : index === 1
                ? 'pending'
                : 'cannot_link',
        })),
  }));

export const channels: Handlers = {
  'sync.get': config,

  'sync.set': (p) => {
    const next = (p.targets ?? {}) as Record<string, unknown>;
    targets = {
      files: strings(next, 'files'),
      folders: strings(next, 'folders'),
    };
    return config();
  },

  'sync.status': () => ({ instances: status() }),

  // Adopting links the instance's own folders into the shared store; the
  // result is what is linked afterwards.
  'instance.sync.adopt': (p) => {
    const wanted = strings(p, 'targets');
    entries.findInstance(str(p, 'instance'));
    return { adopted: wanted.length > 0 ? wanted : targets.folders };
  },

  'instance.sync.share': (p) => {
    const instance = entries.findInstance(str(p, 'instance'));
    const enabled = p.enabled === true;
    if (enabled) opted_out.delete(instance.id);
    else opted_out.add(instance.id);
    return { enabled, warnings: [] };
  },
};
