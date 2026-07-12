export const CONTENT_KINDS = ["mods", "modpacks", "resourcepacks", "shaders"] as const;

export type ContentKind = (typeof CONTENT_KINDS)[number];

export function parseContentKind(value: unknown): ContentKind {
  return CONTENT_KINDS.includes(value as ContentKind) ? (value as ContentKind) : "mods";
}
