import {
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';

import { cn } from '@/lib/utils';

const FADE = '1.75rem';

/** Edge fades cueing scroll direction, so a hidden scrollbar loses no signal. */
function edgeMask(up: boolean, down: boolean): string | undefined {
  if (!up && !down) return undefined;
  const top = up ? 'transparent' : 'black';
  const bottom = down ? 'transparent' : 'black';
  return `linear-gradient(to bottom, ${top}, black ${FADE}, black calc(100% - ${FADE}), ${bottom})`;
}

/**
 * A picker layout for a bounded-height container: the `header` (search, filter
 * chips, an import button) stays fixed while only the region below it scrolls,
 * so those controls never scroll away with the list. The scroll region fills
 * the remaining space, so the parent must have a definite height — e.g. a flex
 * column with a `max-h-*` (as the install modal's stepped body does).
 *
 * The scrollbar is hidden; scrollability is cued instead by a fade at whichever
 * edge has more content past it, so the list reads clean until there's more to
 * see.
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
  const ref = useRef<HTMLDivElement>(null);
  const [edges, setEdges] = useState({ up: false, down: false });

  const update = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    const up = el.scrollTop > 1;
    const down = el.scrollTop + el.clientHeight < el.scrollHeight - 1;
    setEdges((prev) =>
      prev.up === up && prev.down === down ? prev : { up, down },
    );
  }, []);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    for (const child of Array.from(el.children)) observer.observe(child);
    return () => observer.disconnect();
  }, [update]);

  const mask = edgeMask(edges.up, edges.down);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="shrink-0">{header}</div>
      <div
        ref={ref}
        onScroll={update}
        style={mask ? { maskImage: mask, WebkitMaskImage: mask } : undefined}
        className={cn(
          'min-h-0 flex-1 overflow-y-auto overflow-x-hidden scrollbar-none [&::-webkit-scrollbar]:hidden',
          className,
        )}
      >
        {children}
      </div>
    </div>
  );
}
