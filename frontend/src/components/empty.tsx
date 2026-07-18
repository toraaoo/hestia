import type { ReactNode } from 'react';

import { cn } from '@/lib/utils';

/** A dashed empty-state placeholder for lists and tabs with nothing to show. */
export function Empty({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <p
      className={cn(
        'flex items-center justify-center border border-dashed border-border px-4 py-10 text-center text-xs text-muted-foreground',
        className,
      )}
    >
      {children}
    </p>
  );
}
