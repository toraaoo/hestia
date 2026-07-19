import type { ReactNode } from 'react';

import { cn } from '@/lib/utils';

/**
 * A picker layout for a bounded-height container: the `header` (search, filter
 * chips, an import button) stays fixed while only the region below it scrolls,
 * so those controls never scroll away with the list. The scroll region fills
 * the remaining space, so the parent must have a definite height — e.g. a flex
 * column with a `max-h-*` (as the install modal's stepped body does).
 */
export function PickerPanel({
  header,
  className,
  children,
}: {
  header: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="shrink-0">{header}</div>
      <div
        className={cn(
          'min-h-0 flex-1 overflow-x-hidden overflow-y-auto',
          className,
        )}
      >
        {children}
      </div>
    </div>
  );
}
