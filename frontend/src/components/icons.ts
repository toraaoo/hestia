import type { Icon } from '@phosphor-icons/react';
import {
  CubeIcon,
  DatabaseIcon,
  HardDrivesIcon,
  ImagesIcon,
  PackageIcon,
  PlugIcon,
  PuzzlePieceIcon,
  SparkleIcon,
  StackIcon,
} from '@phosphor-icons/react';

import type { ContentKind } from '@/api';
import { m } from '@/paraglide/messages.js';

/** Icon for a library entry kind — instances, hosted servers, profiles. */
export function entryIcon(kind: 'instance' | 'server' | 'profile'): Icon {
  if (kind === 'server') return HardDrivesIcon;
  if (kind === 'profile') return StackIcon;
  return CubeIcon;
}

/** Icon for a piece of content, so the type reads at a glance. */
export function contentIcon(kind: ContentKind): Icon {
  switch (kind) {
    case 'mod':
      return PuzzlePieceIcon;
    case 'resource_pack':
      return ImagesIcon;
    case 'shader':
      return SparkleIcon;
    case 'data_pack':
      return DatabaseIcon;
    case 'modpack':
      return PackageIcon;
    case 'plugin':
      return PlugIcon;
  }
}

export const contentKindLabel: Record<ContentKind, () => string> = {
  mod: m['domain.kind.mod'],
  resource_pack: m['domain.kind.resourcepack'],
  shader: m['domain.kind.shader'],
  data_pack: m['domain.kind.datapack'],
  modpack: m['domain.kind.modpack'],
  plugin: m['domain.kind.plugin'],
};
