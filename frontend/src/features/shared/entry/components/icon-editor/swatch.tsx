/**
 * The one selectable tile the icon editor reuses for every choice: a square
 * swatch (a gradient or a symbol image). Selection follows the skin picker's
 * card language — a quiet outline, no badge.
 */
import type { CSSProperties, ReactNode } from 'react';

import { cn } from '@/lib/utils';

export function IconOptionTile({
  selected,
  onSelect,
  label,
  style,
  className,
  children,
}: {
  selected: boolean;
  onSelect: () => void;
  label: string;
  style?: CSSProperties;
  className?: string;
  children?: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={selected}
      onClick={onSelect}
      style={style}
      className={cn(
        'relative aspect-square overflow-hidden border outline-none transition-colors focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
        selected ? 'border-ember' : 'border-border hover:border-ember/40',
        className,
      )}
    >
      {children}
    </button>
  );
}
