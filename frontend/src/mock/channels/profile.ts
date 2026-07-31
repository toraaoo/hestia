/**
 * `profile.*` — global content profiles: data-home-level lists of project
 * references. Applying one into an instance is `instance.profile.apply`, a
 * content job that lives with the instance channels.
 */
import type { GlobalProfile } from '@/api/types';

import * as content from '../state/content';
import { fail, type Handlers, ok, slug, str, strings } from '../support';

const reference = (ref: string) => {
  const project = content.find(ref);
  return { source: project.source, projectId: project.id, slug: project.slug };
};

const profiles: GlobalProfile[] = [
  {
    name: 'performance',
    entries: [reference('sodium'), reference('lithium')],
  },
];

function find(name: string): GlobalProfile {
  const found = profiles.find((profile) => profile.name === name);
  if (!found) fail('not_found', `no such profile: ${name}`);
  return found;
}

export const channels: Handlers = {
  'profile.list': () => ({ profiles }),

  'profile.create': (p) => {
    const profile: GlobalProfile = { name: slug(str(p, 'name')), entries: [] };
    profiles.push(profile);
    return profile;
  },

  'profile.remove': (p) => {
    const profile = find(str(p, 'name'));
    profiles.splice(profiles.indexOf(profile), 1);
    return ok();
  },

  'profile.edit': (p) => {
    const profile = find(str(p, 'name'));
    const removed = new Set(strings(p, 'remove'));
    profile.entries = [
      ...profile.entries.filter(
        (entry) => !removed.has(entry.slug) && !removed.has(entry.projectId),
      ),
      ...strings(p, 'add').map(reference),
    ];
    return profile;
  },
};
