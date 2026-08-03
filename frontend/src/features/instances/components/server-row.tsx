import {
  ArrowDownIcon,
  ArrowUpIcon,
  CopyIcon,
  DotsSixVerticalIcon,
  DotsThreeIcon,
  HardDrivesIcon,
  PencilSimpleIcon,
  PlayIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { Reorder, type Transition, useDragControls } from 'motion/react';
import { toast } from 'sonner';

import type { ServerEntry } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { pngSource, Thumbnail } from '@/components/ui/thumbnail';
import { m } from '@/paraglide/messages.js';
import { instanceQueries } from '@/queries/instance';

/** A row travels one row-height, so Motion's 450ms layout default reads as lag. */
const REORDER_TRANSITION: Transition = {
  layout: { duration: 0.15, ease: [0.2, 0, 0, 1] },
};

/**
 * One entry of the instance's multiplayer list, in the shape the in-game list
 * shows it: the server's own icon and MOTD as it answers right now, over the
 * address the game will dial.
 *
 * Dragging is a pointer affordance on the handle alone — the row is full of
 * buttons, and a drag that starts on one would be a click the player meant.
 * The same reordering is on the menu, which is the keyboard's way in.
 */
export function ServerRow({
  server,
  joinable,
  playing,
  first,
  last,
  onPlay,
  onEdit,
  onRemove,
  onMoveUp,
  onMoveDown,
  onDragStart,
  onDragEnd,
}: {
  server: ServerEntry;
  joinable: boolean;
  playing: boolean;
  first: boolean;
  last: boolean;
  onPlay: () => void;
  onEdit: () => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onDragStart: () => void;
  onDragEnd: () => void;
}) {
  const status = useQuery(instanceQueries.serverStatus(server.address));
  const controls = useDragControls();
  const online = status.isSuccess;

  return (
    <Reorder.Item
      as="div"
      value={`${server.name}:${server.address}`}
      transition={REORDER_TRANSITION}
      dragListener={false}
      dragControls={controls}
      dragMomentum={false}
      whileDrag={{ scale: 1.01 }}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      className="relative bg-background"
    >
      <div className="flex items-center gap-3 px-3 py-3">
        <button
          type="button"
          aria-hidden
          tabIndex={-1}
          onPointerDown={(event) => controls.start(event)}
          className="shrink-0 cursor-grab touch-none text-muted-foreground/40 transition-colors hover:text-muted-foreground active:cursor-grabbing"
        >
          <DotsSixVerticalIcon className="size-4" />
        </button>

        {/* The live favicon leads: `servers.dat` caches one only for a server
            the game itself has connected to, so it is absent for an entry
            added from here and stale for one that has changed its icon. */}
        <Thumbnail
          src={pngSource(status.data?.favicon, server.icon)}
          icon={HardDrivesIcon}
          size="xl"
        />

        <div className="min-w-0 flex-1 space-y-1">
          <div className="flex items-center gap-2">
            <StatusDot tone={online ? 'on' : 'off'} />
            <span className="truncate text-sm">{server.name}</span>
            {server.acceptTextures && (
              <Badge variant="outline" className="shrink-0">
                {m['instance.servers.textures']()}
              </Badge>
            )}
          </div>
          <p className="truncate font-mono text-[11px] text-muted-foreground">
            {server.address}
          </p>
          <p className="line-clamp-2 text-[11px] text-muted-foreground/80">
            {status.isPending
              ? m['instance.servers.checking']()
              : online
                ? status.data.motd
                : m['instance.servers.offline']()}
          </p>
        </div>

        {online && (
          <div className="shrink-0 space-y-1 text-right text-[11px] text-muted-foreground">
            <div>
              {m['instance.servers.players']({
                online: status.data.playersOnline,
                max: status.data.playersMax,
              })}
            </div>
            <div className="font-mono">{status.data.version}</div>
          </div>
        )}

        <Button
          size="sm"
          data-icon="inline-start"
          className="shrink-0"
          disabled={playing || !joinable}
          title={joinable ? undefined : m['instance.quick_play.unsupported']()}
          onClick={onPlay}
        >
          {playing ? <Spinner /> : <PlayIcon weight="fill" />}
          {m['app.action.join']()}
        </Button>

        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="ghost"
                size="icon-sm"
                className="shrink-0"
                aria-label={m['app.action.more']()}
              >
                <DotsThreeIcon weight="bold" className="size-4" />
              </Button>
            }
          />
          <DropdownMenuContent align="end" className="w-52">
            <DropdownMenuItem onClick={onEdit}>
              <PencilSimpleIcon />
              {m['app.action.edit']()}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() =>
                navigator.clipboard
                  .writeText(server.address)
                  .then(() => toast.success(m['app.toast.copied']()))
              }
            >
              <CopyIcon />
              {m['instance.servers.copy_address']()}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem disabled={first} onClick={onMoveUp}>
              <ArrowUpIcon />
              {m['app.action.move_up']()}
            </DropdownMenuItem>
            <DropdownMenuItem disabled={last} onClick={onMoveDown}>
              <ArrowDownIcon />
              {m['app.action.move_down']()}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onClick={onRemove}>
              <TrashIcon />
              {m['app.action.remove']()}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </Reorder.Item>
  );
}
