import type { Icon } from '@phosphor-icons/react';
import { CheckIcon } from '@phosphor-icons/react';

import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

/**
 * A selectable row shared by every modal that picks content or targets — the
 * install wizard's pick steps, a profile's member selection, the apply
 * picker. Works single- or multi-select: the caller owns the selection and
 * `onSelect` fires on every click (toggle it for multi-select).
 */
export function PickRow({
  icon: RowIcon,
  title,
  subtitle,
  badge,
  disabled,
  selected,
  onSelect,
}: {
  icon: Icon;
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
        'flex items-center gap-3 p-3 text-left ring-1 transition-colors outline-none focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-55',
        selected
          ? 'bg-muted ring-ember'
          : 'ring-border hover:bg-muted/60 hover:ring-foreground/20',
      )}
    >
      <RowIcon className="size-4.5 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{title}</span>
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
