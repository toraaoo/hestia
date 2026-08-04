import type { Icon } from '@phosphor-icons/react';
import { CheckIcon } from '@phosphor-icons/react';

import { CardGridSkeleton } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Thumbnail } from '@/components/ui/thumbnail';
import { cn } from '@/lib/utils';

/**
 * A selectable row shared by every modal that picks content or targets — the
 * install wizard's pick steps, a profile's member selection, the apply
 * picker. Works single- or multi-select: the caller owns the selection and
 * `onSelect` fires on every click (toggle it for multi-select). A selected row
 * highlights whole and carries a trailing check.
 */
export function PickRow({
  icon: RowIcon,
  imageUrl,
  title,
  subtitle,
  badge,
  disabled,
  selected,
  onSelect,
}: {
  icon: Icon;
  /** The content's artwork; falls back to `icon` when absent or on load error. */
  imageUrl?: string;
  title: string;
  subtitle: string;
  badge?: string;
  disabled?: boolean;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      disabled={disabled}
      onClick={onSelect}
      className={cn(
        'flex items-center gap-3 border p-3 text-left transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-55',
        selected
          ? 'border-ember bg-ember/5'
          : 'border-border hover:border-foreground/20 hover:bg-muted/60',
      )}
    >
      <Thumbnail src={imageUrl} icon={RowIcon} size="md" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="min-w-0 truncate text-sm font-medium">{title}</span>
          {badge && (
            <Badge variant="outline" className="shrink-0">
              {badge}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {subtitle}
        </div>
      </div>
      {selected && <CheckIcon weight="bold" className="size-4 text-ember" />}
    </button>
  );
}

export function PickRowSkeleton({ count }: { count: number }) {
  return (
    <CardGridSkeleton
      grid="grid gap-2 p-0.5"
      count={count}
      card="h-[4.25rem]"
    />
  );
}
