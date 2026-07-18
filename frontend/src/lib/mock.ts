/**
 * The cross-feature slice of the static stand-in vocabulary: the content
 * kinds every feature labels content with. Feature-specific stand-ins live in
 * each feature's own `mock.ts`. Nothing talks to a backend.
 */

export type ContentKind =
  | 'mod'
  | 'resourcepack'
  | 'shader'
  | 'datapack'
  | 'modpack';
