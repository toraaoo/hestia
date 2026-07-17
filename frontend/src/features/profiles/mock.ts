/**
 * Static stand-in global profiles, shaped after the daemon's `profile.list`
 * surface (data-home-level project reference lists). Nothing talks to a
 * backend.
 */

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
