/** Mirrors `crates/proto/src/profile.rs`. */

/** One project reference of a global profile. */
export interface ProfileEntry {
  source: string;
  projectId: string;
  slug: string;
}

/**
 * A global content profile: a data-home-level project reference list, applied
 * into an instance's pool as ordinary tagged content. References, never jars —
 * each apply resolves per instance.
 */
export interface GlobalProfile {
  name: string;
  entries: ProfileEntry[];
}
