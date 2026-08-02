import type { Icon } from '@phosphor-icons/react';
import type { ReactNode } from 'react';

import { cn } from '@/lib/utils';

/**
 * A borderless empty state: a muted glyph over a message, optionally a
 * supporting line and one action. Centred in whatever space it is given, so a
 * parent that stretches it (`h-full`) centres it vertically too.
 */
export function Empty({
  icon: Glyph,
  children,
  description,
  action,
  tone = 'muted',
  className,
}: {
  icon?: Icon;
  children: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
  tone?: 'muted' | 'destructive';
  className?: string;
}) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center gap-4 px-6 py-14 text-center',
        className,
      )}
    >
      {Glyph && (
        <Glyph
          weight="light"
          className={cn(
            'size-8',
            tone === 'destructive'
              ? 'text-destructive/60'
              : 'text-muted-foreground/50',
          )}
        />
      )}
      <div className="max-w-sm space-y-1.5">
        <h3
          className={cn(
            'text-sm font-medium',
            tone === 'destructive' && 'text-destructive',
          )}
        >
          {children}
        </h3>
        {description && (
          <p className="text-xs text-muted-foreground">{description}</p>
        )}
      </div>
      {action}
    </div>
  );
}
