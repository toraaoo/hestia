import type { Icon } from '@phosphor-icons/react';
import { useState } from 'react';

import { cn } from '@/lib/utils';

const sizes = {
  xs: { box: 'size-6', glyph: 'size-3.5' },
  sm: { box: 'size-7', glyph: 'size-4' },
  md: { box: 'size-8', glyph: 'size-4.5' },
  lg: { box: 'size-9', glyph: 'size-5' },
  xl: { box: 'size-12', glyph: 'size-6' },
} as const;

/** The square image-or-glyph tile a list row or card leads with. */
export function Thumbnail({
  src,
  icon: Glyph,
  size = 'sm',
  className,
}: {
  /** A URL or data URI; the glyph stands in when it is absent or fails. */
  src?: string;
  icon: Icon;
  size?: keyof typeof sizes;
  className?: string;
}) {
  // By source, not a flag: a tile whose image changes underneath it must get
  // a fresh attempt.
  const [broken, setBroken] = useState<string | null>(null);
  const { box, glyph } = sizes[size];

  if (!src || broken === src) {
    return (
      <span
        className={cn(
          'grid shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border',
          box,
          className,
        )}
      >
        <Glyph className={glyph} />
      </span>
    );
  }

  return (
    <img
      src={src}
      alt=""
      onError={() => setBroken(src)}
      className={cn('shrink-0 object-cover ring-1 ring-border', box, className)}
    />
  );
}

/**
 * The first of `sources` that carries anything, as an inline PNG. Locally-read
 * icons travel as bare base64 rather than as a path, so the webview's asset
 * protocol never has to reach into the data home.
 */
export function pngSource(...sources: (string | undefined)[]) {
  const source = sources.find((candidate) => !!candidate);
  return source ? `data:image/png;base64,${source}` : undefined;
}
