import { GlobeHemisphereWestIcon } from '@phosphor-icons/react';
import { useState } from 'react';
import type { WorldInfo } from '@/api';
import { cn } from '@/lib/utils';

/**
 * A world's own in-game thumbnail (`icon.png`), which the daemon inlines as
 * base64 rather than as a path — serving it as a file would mean widening the
 * webview's asset-protocol reach to the whole data home. Falls back to a globe
 * both for a world the game never rendered a preview of and for one whose
 * thumbnail will not decode, as the content rows do for a project icon.
 */
export function WorldIcon({
  world,
  className,
}: {
  world: WorldInfo;
  className?: string;
}) {
  const [broken, setBroken] = useState(false);
  if (!world.icon || broken) {
    return (
      <span
        className={cn(
          'grid size-8 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border',
          className,
        )}
      >
        <GlobeHemisphereWestIcon className="size-4" />
      </span>
    );
  }
  return (
    <img
      src={`data:image/png;base64,${world.icon}`}
      alt=""
      onError={() => setBroken(true)}
      className={cn(
        'size-8 shrink-0 object-cover ring-1 ring-border',
        className,
      )}
    />
  );
}
