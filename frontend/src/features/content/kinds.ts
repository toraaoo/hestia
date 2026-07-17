import type { ContentKind } from '@/lib/mock';

export type KindSlug =
  | 'mods'
  | 'modpacks'
  | 'resourcepacks'
  | 'shaders'
  | 'datapacks';

export const kindInfo: Record<ContentKind, { slug: KindSlug; label: string }> =
  {
    mod: { slug: 'mods', label: 'Mods' },
    modpack: { slug: 'modpacks', label: 'Modpacks' },
    resourcepack: { slug: 'resourcepacks', label: 'Resource packs' },
    shader: { slug: 'shaders', label: 'Shaders' },
    datapack: { slug: 'datapacks', label: 'Datapacks' },
  };

export const contentKinds = Object.keys(kindInfo) as ContentKind[];

export const kindBySlug = (slug: string): ContentKind | undefined =>
  contentKinds.find((kind) => kindInfo[kind].slug === slug);
