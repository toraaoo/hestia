import { useVirtualizer } from '@tanstack/react-virtual';
import { type HTMLAttributes, useEffect, useRef } from 'react';

import { cn } from '@/lib/utils';

export interface LogRow {
  /** The line's text; rendered pre-wrapped in a monospace row. */
  text: string;
  /** Extra classes for this row (console echo/error colouring). */
  className?: string;
}

/**
 * A tail-following log surface that windows the DOM: only the rows in view
 * mount, so a thousand-line buffer costs O(viewport) instead of O(buffer) and a
 * startup dump never stalls the main thread. Streaming stays live — new rows
 * append at the tail and the view follows them, but only while the user is
 * already at the bottom, so scrolling up to read is never interrupted.
 */
export function LogView({
  rows,
  className,
  emptyLabel,
  ...rest
}: {
  rows: LogRow[];
  className?: string;
  emptyLabel?: string;
} & HTMLAttributes<HTMLDivElement>) {
  const parentRef = useRef<HTMLDivElement>(null);
  const atBottom = useRef(true);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 16,
    overscan: 24,
  });

  const onScroll = () => {
    const el = parentRef.current;
    if (!el) return;
    atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 8;
  };

  // Follow the tail on new rows, but only when pinned to the bottom. rAF defers
  // the scroll past the virtualizer's layout for this frame so it never forces
  // a synchronous reflow per batch.
  useEffect(() => {
    if (!atBottom.current || rows.length === 0) return;
    const raf = requestAnimationFrame(() => {
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end' });
    });
    return () => cancelAnimationFrame(raf);
  }, [rows.length, virtualizer]);

  const items = virtualizer.getVirtualItems();

  return (
    <div
      ref={parentRef}
      onScroll={onScroll}
      className={cn(
        'min-h-0 flex-1 overflow-y-auto border border-border bg-card p-3 font-mono text-[11px] text-muted-foreground',
        className,
      )}
      {...rest}
    >
      {rows.length === 0 && emptyLabel ? (
        <span className="text-muted-foreground/60">{emptyLabel}</span>
      ) : (
        <div
          className="relative w-full"
          style={{ height: virtualizer.getTotalSize() }}
        >
          {items.map((item) => (
            <div
              key={item.key}
              data-index={item.index}
              ref={virtualizer.measureElement}
              className={cn(
                'absolute top-0 left-0 w-full wrap-break-word whitespace-pre-wrap',
                rows[item.index].className,
              )}
              style={{ transform: `translateY(${item.start}px)` }}
            >
              {rows[item.index].text}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
