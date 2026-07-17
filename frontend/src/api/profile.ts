/**
 * The `profile.*` channels — global content profiles: data-home-level project
 * reference lists. Applying one into an instance is `instance.profile.apply`,
 * a content job (see `instance.applyProfile`).
 */

import { call } from './core/ipc';
import type { GlobalProfile } from './types/profile';

export async function list(): Promise<GlobalProfile[]> {
  const result = await call<{ profiles: GlobalProfile[] }>('profile.list');
  return result.profiles;
}

/** The name is slugged (`My QoL` becomes `my-qol`). */
export function create(name: string): Promise<GlobalProfile> {
  return call('profile.create', { name });
}

export async function remove(name: string): Promise<void> {
  await call('profile.remove', { name });
}

/**
 * Add/remove project references (slugs or ids); adds resolve through the
 * content registry on `source` (empty = the default source).
 */
export function edit(
  name: string,
  options: { source?: string; add?: string[]; remove?: string[] },
): Promise<GlobalProfile> {
  return call(
    'profile.edit',
    {
      name,
      source: options.source ?? '',
      add: options.add ?? [],
      remove: options.remove ?? [],
    },
    { timeoutMs: 60_000 },
  );
}
