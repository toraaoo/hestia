/**
 * Static stand-in global profiles, shaped after the daemon's `profile.list`
 * surface (data-home-level project reference lists). Nothing talks to a
 * backend.
 */

import { getProject } from '@/features/content/mock';

/** One project reference of a global profile. */
export interface GlobalProfileEntry {
  source: string;
  project_id: string;
  slug: string;
}

/**
 * A reusable content list applied into any instance — references, never
 * jars: each apply resolves against the target instance's version and loader.
 */
export interface GlobalProfile {
  name: string;
  entries: GlobalProfileEntry[];
}

/** The content kinds a profile's references resolve to, deduplicated. */
export function profileKinds(profile: GlobalProfile) {
  const kinds = profile.entries.map(
    (e) => getProject(e.slug)?.kind ?? ('mod' as const),
  );
  return [...new Set(kinds)];
}

export const globalProfiles: GlobalProfile[] = [
  {
    name: 'performance',
    entries: [
      { source: 'modrinth', project_id: 'sodium', slug: 'sodium' },
      { source: 'modrinth', project_id: 'lithium', slug: 'lithium' },
      { source: 'modrinth', project_id: 'iris', slug: 'iris' },
    ],
  },
  {
    name: 'qol',
    entries: [
      { source: 'modrinth', project_id: 'voice-chat', slug: 'voice-chat' },
      { source: 'modrinth', project_id: 'faithful', slug: 'faithful' },
    ],
  },
];
