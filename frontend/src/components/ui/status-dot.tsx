import { cn } from '@/lib/utils';

/** A small square state light. Square to match the base-lyra (rounded-none) UI. */
const tones = {
  on: 'bg-ember',
  warn: 'bg-amber',
  off: 'bg-muted-foreground/40',
} as const;

export function StatusDot({
  tone = 'off',
  className,
}: {
  tone?: keyof typeof tones;
  className?: string;
}) {
  return <span className={cn('size-2 shrink-0', tones[tone], className)} />;
}
