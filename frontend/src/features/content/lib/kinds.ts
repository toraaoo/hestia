import type { ContentKind } from '@/api';
import { m } from '@/paraglide/messages.js';

export type KindSlug =
  | 'mods'
  | 'modpacks'
  | 'resourcepacks'
  | 'shaders'
  | 'datapacks'
  | 'plugins';

export const kindInfo: Record<
  ContentKind,
  { slug: KindSlug; label: () => string }
> = {
  mod: { slug: 'mods', label: m['domain.kind.mods'] },
  modpack: { slug: 'modpacks', label: m['domain.kind.modpacks'] },
  resource_pack: {
    slug: 'resourcepacks',
    label: m['domain.kind.resourcepacks'],
  },
  shader: { slug: 'shaders', label: m['domain.kind.shaders'] },
  data_pack: { slug: 'datapacks', label: m['domain.kind.datapacks'] },
  plugin: { slug: 'plugins', label: m['domain.kind.plugins'] },
};

/**
 * Every kind the wire defines. Only for validating a URL search param, where
 * the entry — and so the set it actually accepts — is not loaded yet; an entry
 * carries its own `accepts` and that is what a surface renders.
 */
export const isContentKind = (value: unknown): value is ContentKind =>
  typeof value === 'string' && value in kindInfo;

export const contentKinds = Object.keys(kindInfo) as ContentKind[];

export const kindBySlug = (slug: string): ContentKind | undefined =>
  contentKinds.find((kind) => kindInfo[kind].slug === slug);
