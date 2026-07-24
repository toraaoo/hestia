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
  bust,
  className,
}: {
  uuid: string;
  name: string;
  /** Rendered edge length in pixels. */
  size?: number;
  /**
   * Cache-bust token appended to the head url. The mc-heads url is uuid-only,
   * so a skin change reuses it and the browser serves the stale head; pass the
   * equipped skin's key here to force a re-fetch when it changes.
   */
  bust?: string;
  className?: string;
}) {
  // Track the identity whose head failed, so switching account — or equipping a
  // new skin — re-tries the new head instead of staying on the fallback.
  const id = bust ? `${uuid}:${bust}` : uuid;
  const [failedId, setFailedId] = useState<string | null>(null);
  const failed = failedId === id;

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
      src={`https://mc-heads.net/avatar/${uuid}/${size * 2}${
        bust ? `?v=${encodeURIComponent(bust)}` : ''
      }`}
      alt=""
      width={size}
      height={size}
      style={style}
      onError={() => setFailedId(id)}
      className={cn(box, 'bg-muted [image-rendering:pixelated]')}
    />
  );
}
