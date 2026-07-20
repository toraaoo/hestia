import type { Icon } from '@phosphor-icons/react';
import { CheckIcon, PlusIcon } from '@phosphor-icons/react';

import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

/**
 * A selectable row shared by every modal that picks content or targets — the
 * install wizard's pick steps, a profile's member selection, the apply
 * picker. Works single- or multi-select: the caller owns the selection and
 * `onSelect` fires on every click (toggle it for multi-select).
 *
 * The `select` variant highlights the whole row (a committed choice); the
 * `add` variant keeps the row flat with a trailing add/added affordance, for
 * search lists that only feed a selection reviewed elsewhere.
 */
export function PickRow({
  icon: RowIcon,
  title,
  subtitle,
  badge,
  disabled,
  selected,
  onSelect,
  variant = 'select',
}: {
  icon: Icon;
  title: string;
  subtitle: string;
  badge?: string;
  disabled?: boolean;
  selected: boolean;
  onSelect: () => void;
  variant?: 'select' | 'add';
}) {
  const highlight = variant === 'select' && selected;
  return (
    <button
      type="button"
      aria-pressed={selected}
      disabled={disabled}
      onClick={onSelect}
      className={cn(
        'flex items-center gap-3 border p-3 text-left transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-55',
        highlight
          ? 'border-ember bg-ember/5'
          : 'border-border hover:border-foreground/20 hover:bg-muted/60',
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
      {variant === 'add' ? (
        <span
          className={cn(
            'flex size-6 shrink-0 items-center justify-center border transition-colors',
            selected
              ? 'border-ember text-ember'
              : 'border-border text-muted-foreground',
          )}
        >
          {selected ? (
            <CheckIcon weight="bold" className="size-3.5" />
          ) : (
            <PlusIcon className="size-3.5" />
          )}
        </span>
      ) : (
        selected && <CheckIcon weight="bold" className="size-4 text-ember" />
      )}
    </button>
  );
}
