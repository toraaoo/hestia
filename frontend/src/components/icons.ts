import type { Icon } from '@phosphor-icons/react';
import {
  CubeIcon,
  DatabaseIcon,
  HardDrivesIcon,
  ImagesIcon,
  PackageIcon,
  PuzzlePieceIcon,
  SparkleIcon,
} from '@phosphor-icons/react';

import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

/** Icon for a library entry kind — instances vs hosted servers. */
export function entryIcon(kind: 'instance' | 'server'): Icon {
  return kind === 'server' ? HardDrivesIcon : CubeIcon;
}

/** Icon for a piece of content, so the type reads at a glance. */
export function contentIcon(kind: ContentKind): Icon {
  switch (kind) {
    case 'mod':
      return PuzzlePieceIcon;
    case 'resourcepack':
      return ImagesIcon;
    case 'shader':
      return SparkleIcon;
    case 'datapack':
      return DatabaseIcon;
    case 'modpack':
      return PackageIcon;
  }
}

export const contentKindLabel: Record<ContentKind, () => string> = {
  mod: m['kind.mod'],
  resourcepack: m['kind.resourcepack'],
  shader: m['kind.shader'],
  datapack: m['kind.datapack'],
  modpack: m['kind.modpack'],
};
