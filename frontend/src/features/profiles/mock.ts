/**
 * Static stand-in global profiles, shaped after the daemon's `profile.list`
 * surface (data-home-level project reference lists). Nothing talks to a
 * backend.
 */

/** One project reference of a global profile. */
export interface GlobalProfileEntry {
  source: string;
  projectId: string;
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

export const globalProfiles: GlobalProfile[] = [
  {
    name: 'performance',
    entries: [
      { source: 'modrinth', projectId: 'sodium', slug: 'sodium' },
      { source: 'modrinth', projectId: 'lithium', slug: 'lithium' },
      { source: 'modrinth', projectId: 'iris', slug: 'iris' },
    ],
  },
  {
    name: 'qol',
    entries: [
      { source: 'modrinth', projectId: 'voice-chat', slug: 'voice-chat' },
      { source: 'modrinth', projectId: 'faithful', slug: 'faithful' },
    ],
  },
];
