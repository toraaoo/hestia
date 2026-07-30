import type { ReactNode } from 'react';
import type { ContentKind, ContentVersion, InstalledContent } from '@/api';

/** The entry a content tab acts on. */
export interface EntryTarget {
  kind: 'server' | 'instance';
  id: string;
  flavor: string;
  gameVersion: string;
}

/** How the daemon matches an item: its project id, else its filename. */
export const installedRef = (i: InstalledContent) => i.projectId || i.filename;

/** The installed pool narrowed by the kind filter and the quick search. */
export const filterContent = (
  items: InstalledContent[],
  kind: ContentKind | undefined,
  search: string,
): InstalledContent[] => {
  const q = search.trim().toLowerCase();
  return items.filter(
    (i) =>
      (!kind || i.kind === kind) &&
      (!q ||
        i.title.toLowerCase().includes(q) ||
        i.filename.toLowerCase().includes(q)),
  );
};

/** A stable identity for one installed row; the index keys an item by filename. */
export const rowKey = (i: InstalledContent) => `${i.kind}:${i.filename}`;

/**
 * The index stores a world data-relative (`saves/My World`) while the wire's
 * `worlds` scope names it bare — one conversion, here.
 */
export const worldName = (world: string) => world.split('/').pop() ?? world;

/**
 * The worlds a datapack loads in: its own selection, or — when it has none —
 * every world the entry has, now or later. Mirrors `install::target_worlds`.
 */
export const packWorlds = (
  item: InstalledContent,
  entryWorlds: string[],
): string[] =>
  item.worlds.length > 0 ? item.worlds.map(worldName) : entryWorlds;

/** Whether a pack is loaded in one world: the per-world twin of `enabled`. */
export const worldEnabled = (item: InstalledContent, world: string): boolean =>
  item.enabled && !item.disabledWorlds.map(worldName).includes(world);

/** The loader filter a kind's version lookup needs, given the entry's flavor. */
export const kindLoader = (
  kind: ContentKind,
  flavor: string,
): string | undefined =>
  kind === 'mod' || kind === 'plugin'
    ? flavor
    : kind === 'data_pack'
      ? 'datapack'
      : undefined;

export interface RowHandlers {
  /** `worlds` narrows a datapack to those saves; omitted covers every one. */
  onEnable: (
    item: InstalledContent,
    enabled: boolean,
    worlds?: string[],
  ) => void;
  onRemove: (item: InstalledContent, worlds?: string[]) => void;
  onUpdate: (item: InstalledContent) => void;
  onSetVersion: (item: InstalledContent, version: ContentVersion) => void;
}

export interface SectionProps {
  entry: EntryTarget;
  kinds: ContentKind[];
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
  action?: ReactNode;
}

export type ListResult = {
  data?: { items: InstalledContent[]; untracked: string[] };
};
export type UpdatesResult = {
  data?: { filename: string; updatable: boolean }[];
  isFetching: boolean;
  refetch: () => void;
};
