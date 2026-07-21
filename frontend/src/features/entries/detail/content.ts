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

/** A stable identity for one installed row (a datapack repeats per world). */
export const rowKey = (i: InstalledContent) =>
  `${i.kind}:${i.filename}:${i.world}`;

/** The world folder to narrow a datapack toggle/removal to (else none). */
export const itemWorlds = (i: InstalledContent): string[] =>
  i.kind === 'data_pack' && i.world
    ? [i.world.split('/').pop() ?? i.world]
    : [];

/** The loader filter a kind's version lookup needs, given the entry's flavor. */
export const kindLoader = (
  kind: ContentKind,
  flavor: string,
): string | undefined =>
  kind === 'mod' ? flavor : kind === 'data_pack' ? 'datapack' : undefined;

export interface RowHandlers {
  onEnable: (item: InstalledContent, enabled: boolean) => void;
  onRemove: (item: InstalledContent) => void;
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
