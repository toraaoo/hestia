import { useState } from 'react';

import { cn } from '@/lib/utils';

/**
 * A Minecraft player-head avatar, rendered from the public mc-heads service by
 * account uuid (helm overlay included) — the same source Modrinth's launcher
 * uses. Falls back to the name's initials when there is no uuid or the head
 * fails to load (offline, unknown profile).
 */
export function AccountAvatar({
  uuid,
  name,
  size = 28,
  className,
}: {
  uuid: string;
  name: string;
  /** Rendered edge length in pixels. */
  size?: number;
  className?: string;
}) {
  // Track the uuid whose head failed, so switching accounts re-tries the new
  // head instead of staying on the fallback.
  const [failedUuid, setFailedUuid] = useState<string | null>(null);
  const failed = failedUuid === uuid;

  const box = cn('shrink-0 overflow-hidden ring-1 ring-border', className);
  const style = { width: size, height: size };

  if (!uuid || failed) {
    return (
      <span
        style={style}
        className={cn(
          box,
          'grid place-items-center bg-muted font-semibold text-muted-foreground',
        )}
      >
        {name.slice(0, 2).toUpperCase()}
      </span>
    );
  }

  return (
    <img
      src={`https://mc-heads.net/avatar/${uuid}/${size * 2}`}
      alt=""
      width={size}
      height={size}
      style={style}
      onError={() => setFailedUuid(uuid)}
      className={cn(box, 'bg-muted [image-rendering:pixelated]')}
    />
  );
}
