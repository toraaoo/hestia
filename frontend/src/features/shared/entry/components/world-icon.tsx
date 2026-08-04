import { GlobeHemisphereWestIcon } from '@phosphor-icons/react';

import type { WorldInfo } from '@/api';
import { pngSource, Thumbnail } from '@/components/ui/thumbnail';

/** A save's own in-game thumbnail, or a globe for one the game never rendered. */
export function WorldIcon({
  world,
  className,
}: {
  world: WorldInfo;
  className?: string;
}) {
  return (
    <Thumbnail
      src={pngSource(world.icon)}
      icon={GlobeHemisphereWestIcon}
      size="md"
      className={className}
    />
  );
}
