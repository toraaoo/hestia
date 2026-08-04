import {
  ArrowDownIcon,
  ArrowUpIcon,
  DotsSixVerticalIcon,
  HardDrivesIcon,
} from '@phosphor-icons/react';
import { Reorder, type Transition } from 'motion/react';
import { useState } from 'react';

import type { ServerEntry } from '@/api';
import { Button } from '@/components/ui/button';
import { pngSource, Thumbnail } from '@/components/ui/thumbnail';
import { m } from '@/paraglide/messages.js';

/** A row travels one row-height, so Motion's 450ms layout default reads as lag. */
const REORDER_TRANSITION: Transition = {
  layout: { duration: 0.15, ease: [0.2, 0, 0, 1] },
};

const keyOf = (server: ServerEntry) => `${server.name}:${server.address}`;

/**
 * The list in reorder mode: the same rows stripped to what identifies them,
 * arranged by drag or by the arrows, and committed as one order.
 *
 * The arrows are not a fallback — a pointer drag is unusable from the keyboard
 * and unannounced to a screen reader, so every move goes through the same
 * `move()` and is read out of the live region below the list.
 */
export function ServerReorderList({
  servers,
  onOrder,
}: {
  servers: ServerEntry[];
  /** The arrangement so far, by name (or address, for an unnamed entry). */
  onOrder: (order: string[]) => void;
}) {
  const [rows, setRows] = useState(servers);
  const [announcement, setAnnouncement] = useState('');

  const apply = (next: ServerEntry[]) => {
    setRows(next);
    onOrder(next.map((server) => server.name || server.address));
  };

  const move = (from: number, to: number) => {
    if (to < 0 || to >= rows.length) return;
    const next = [...rows];
    const [server] = next.splice(from, 1);
    next.splice(to, 0, server);
    apply(next);
    setAnnouncement(
      m['instance.servers.reorder.moved']({
        name: server.name || server.address,
        position: to + 1,
        total: next.length,
      }),
    );
  };

  return (
    <>
      <Reorder.Group
        as="div"
        axis="y"
        values={rows}
        onReorder={apply}
        className="divide-y divide-border border border-border"
      >
        {rows.map((server, index) => (
          <Reorder.Item
            as="div"
            key={keyOf(server)}
            value={server}
            transition={REORDER_TRANSITION}
            dragMomentum={false}
            whileDrag={{ scale: 1.01 }}
            className="relative flex cursor-grab items-center gap-3 bg-background px-3 py-2.5 active:cursor-grabbing"
          >
            <DotsSixVerticalIcon className="size-4 shrink-0 text-muted-foreground/50" />
            <Thumbnail
              src={pngSource(server.icon)}
              icon={HardDrivesIcon}
              className="pointer-events-none"
            />
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm">{server.name}</div>
              <div className="truncate font-mono text-[11px] text-muted-foreground">
                {server.address}
              </div>
            </div>
            <Button
              size="icon-xs"
              variant="ghost"
              className="shrink-0"
              aria-label={m['app.action.move_up']()}
              disabled={index === 0}
              onClick={() => move(index, index - 1)}
            >
              <ArrowUpIcon />
            </Button>
            <Button
              size="icon-xs"
              variant="ghost"
              className="shrink-0"
              aria-label={m['app.action.move_down']()}
              disabled={index === rows.length - 1}
              onClick={() => move(index, index + 1)}
            >
              <ArrowDownIcon />
            </Button>
          </Reorder.Item>
        ))}
      </Reorder.Group>
      <p aria-live="polite" className="sr-only">
        {announcement}
      </p>
    </>
  );
}
