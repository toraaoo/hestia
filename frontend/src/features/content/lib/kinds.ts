import type { ContentKind } from '@/api';
import { m } from '@/paraglide/messages.js';

export type KindSlug =
  | 'mods'
  | 'modpacks'
  | 'resourcepacks'
  | 'shaders'
  | 'datapacks';

export const kindInfo: Record<
  ContentKind,
  { slug: KindSlug; label: () => string }
> = {
  mod: { slug: 'mods', label: m['kind.mods'] },
  modpack: { slug: 'modpacks', label: m['kind.modpacks'] },
  resource_pack: { slug: 'resourcepacks', label: m['kind.resourcepacks'] },
  shader: { slug: 'shaders', label: m['kind.shaders'] },
  data_pack: { slug: 'datapacks', label: m['kind.datapacks'] },
};

export const contentKinds = Object.keys(kindInfo) as ContentKind[];

export const kindBySlug = (slug: string): ContentKind | undefined =>
  contentKinds.find((kind) => kindInfo[kind].slug === slug);
