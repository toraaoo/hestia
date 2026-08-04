import { useVirtualizer } from '@tanstack/react-virtual';
import { useMeasure } from '@uidotdev/usehooks';
import { motion } from 'motion/react';
import { useCallback, useEffect, useRef } from 'react';

import type { ContentProject } from '@/api';
import { Bone } from '@/components/skeleton';
import { ContentCard } from '@/features/content/components/content-card';
import { projectKey } from '@/features/content/lib';
import { listContainer, listItem } from '@/lib/motion';

const SECOND_COLUMN_WIDTH = 960;
const ROW_GAP = 12;
const ROW_HEIGHT_ESTIMATE = 96 + ROW_GAP;
const PREFETCH_ROWS = 3;

export function ResultGrid({
  hits,
  loadingMore,
  hasMore,
  onReachEnd,
}: {
  hits: ContentProject[];
  loadingMore: boolean;
  hasMore: boolean;
  onReachEnd: () => void;
}) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const [measureRef, { width }] = useMeasure<HTMLDivElement>();
  const attach = useCallback(
    (node: HTMLDivElement | null) => {
      scrollRef.current = node;
      measureRef(node);
    },
    [measureRef],
  );

  const columns = (width ?? 0) >= SECOND_COLUMN_WIDTH ? 2 : 1;
  const cellsToCompleteRow = (columns - (hits.length % columns)) % columns;
  const cells = hits.length + (loadingMore ? cellsToCompleteRow + columns : 0);
  const rows = Math.ceil(cells / columns);

  const virtualizer = useVirtualizer({
    count: rows,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT_ESTIMATE,
    overscan: 4,
  });

  const items = virtualizer.getVirtualItems();
  const cardRowHeight =
    virtualizer.measurementsCache[0]?.size ?? ROW_HEIGHT_ESTIMATE;
  const tail = items.at(-1)?.index ?? 0;
  useEffect(() => {
    if (hasMore && !loadingMore && tail >= rows - 1 - PREFETCH_ROWS) {
      onReachEnd();
    }
  }, [hasMore, loadingMore, tail, rows, onReachEnd]);

  const entering = useRef(true);

  return (
    <div ref={attach} className="h-full min-h-0 overflow-y-auto">
      <motion.div
        initial="hidden"
        animate="show"
        variants={listContainer(items.length)}
        onAnimationComplete={() => {
          if (items.length > 0) entering.current = false;
        }}
        className="relative w-full"
        style={{ height: virtualizer.getTotalSize() }}
      >
        {items.map((row) => (
          <div
            key={row.key}
            data-index={row.index}
            ref={virtualizer.measureElement}
            className="absolute top-0 left-0 w-full"
            style={{ transform: `translateY(${row.start}px)` }}
          >
            <motion.div
              variants={listItem}
              initial={entering.current ? undefined : false}
              className="grid gap-3 pb-3"
              style={{
                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                height:
                  row.index * columns >= hits.length
                    ? cardRowHeight
                    : undefined,
              }}
            >
              {Array.from({ length: columns }, (_, column) => {
                const index = row.index * columns + column;
                if (index >= cells) return null;
                const project = hits[index];
                return project ? (
                  <ContentCard key={projectKey(project)} project={project} />
                ) : (
                  <Bone key={`bone-${index}`} />
                );
              })}
            </motion.div>
          </div>
        ))}
      </motion.div>
    </div>
  );
}
