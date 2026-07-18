import { cn } from '@/lib/utils';

/**
 * Hand-drawn loading placeholders. `Bone` is the primitive — a pulsing
 * theme-muted block sized by utility classes — and `CardGridSkeleton` covers
 * the card-grid pages by mirroring their real grid classes. Page-shaped
 * compositions live beside the page they stand in for.
 */
export function Bone({ className }: { className?: string }) {
  return (
    <div aria-hidden className={cn('animate-pulse bg-muted', className)} />
  );
}

/** A section-header line plus a grid of card bones. */
export function CardGridSkeleton({
  grid,
  count,
  card,
  header = false,
}: {
  /** The page's real grid classes, so bones land where cards will. */
  grid: string;
  count: number;
  /** Sizing classes for one card bone. */
  card: string;
  header?: boolean;
}) {
  return (
    <div>
      {header && <Bone className="mb-3 h-3 w-24" />}
      <div className={grid}>
        {Array.from({ length: count }, (_, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: static placeholders
          <Bone key={i} className={card} />
        ))}
      </div>
    </div>
  );
}
