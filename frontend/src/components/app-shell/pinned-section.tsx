import {
  CubeIcon,
  HardDrivesIcon,
  PlusIcon,
  PushPinSlashIcon,
} from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useEffect, useMemo, useRef, useState } from 'react';

import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { StatusDot } from '@/components/ui/status-dot';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { useInstances } from '@/queries/instance';
import { type PinnedEntry, pinKey, usePinned } from '@/queries/pinned';
import { serverQueries, useServers } from '@/queries/server';

type ResolvedPin = PinnedEntry & {
  name: string;
  flavor: string;
  version: string;
  running: boolean;
  /** Running session count — instances only; a server is always 0. */
  sessions: number;
  iconUrl?: string;
};

/** The drag-reorder handlers a pinned row wires to its DnD events. */
interface DragProps {
  draggable: boolean;
  dragging: boolean;
  onStart: () => void;
  onEnter: () => void;
  onDrop: () => void;
  onEnd: () => void;
}

/** Local reorder-while-dragging preview; the drop commits it to prefs. */
function usePinnedReorder(
  pinned: ResolvedPin[],
  save: (entries: PinnedEntry[]) => void,
) {
  const [drag, setDrag] = useState<{
    list: ResolvedPin[];
    index: number;
  } | null>(null);

  const rows = drag?.list ?? pinned;
  const begin = (index: number) => setDrag({ list: pinned, index });
  const over = (index: number) =>
    setDrag((current) => {
      if (!current || current.index === index) return current;
      const list = [...current.list];
      const [moved] = list.splice(current.index, 1);
      list.splice(index, 0, moved);
      return { list, index };
    });
  const end = (commit: boolean) => {
    if (commit && drag) save(drag.list.map(({ kind, id }) => ({ kind, id })));
    setDrag(null);
  };

  return {
    rows,
    dragFor: (index: number): DragProps => ({
      draggable: rows.length > 1,
      dragging: drag !== null && drag.index === index,
      onStart: () => begin(index),
      onEnter: () => over(index),
      onDrop: () => end(true),
      onEnd: () => end(false),
    }),
  };
}

export function PinnedSection({ pathname }: { pathname: string }) {
  const instances = useInstances();
  const servers = useServers();
  const { pins: pinnedEntries, ready, isPinned, toggle, save } = usePinned();

  const instanceList = instances.data ?? [];
  const serverList = servers.data ?? [];

  const pinned = useMemo<ResolvedPin[]>(
    () =>
      pinnedEntries.flatMap((pin) => {
        if (pin.kind === 'instance') {
          const entry = instanceList.find((i) => i.id === pin.id);
          if (!entry) return [];
          const sessions = (entry.sessions ?? []).filter(
            (session) => session.state === 'running',
          ).length;
          return [
            {
              ...pin,
              name: entry.name,
              flavor: entry.flavor,
              version: entry.gameVersion,
              running: sessions > 0,
              sessions,
              iconUrl: entry.iconUrl,
            },
          ];
        }
        const entry = serverList.find((s) => s.id === pin.id);
        if (!entry) return [];
        return [
          {
            ...pin,
            name: entry.name,
            flavor: entry.flavor,
            version: entry.gameVersion,
            running: entry.process?.state === 'running',
            sessions: 0,
            iconUrl: entry.iconUrl,
          },
        ];
      }),
    [pinnedEntries, instanceList, serverList],
  );

  // Persist the pruned list when a pinned entry is deleted elsewhere. Both
  // lists must be loaded first, or a still-fetching query reads as empty and
  // would wrongly drop live pins. The ref keeps the effect off `save`'s churn.
  const saveRef = useRef(save);
  saveRef.current = save;
  useEffect(() => {
    if (!ready || !instances.data || !servers.data) return;
    if (pinned.length === pinnedEntries.length) return;
    saveRef.current(pinned.map(({ kind, id }) => ({ kind, id })));
  }, [ready, instances.data, servers.data, pinned, pinnedEntries]);

  const { rows, dragFor } = usePinnedReorder(pinned, save);
  const nothingToPin = instanceList.length === 0 && serverList.length === 0;

  return (
    <div className="border-t border-border p-2">
      <div className="flex items-center justify-between px-3 pt-1 pb-1.5">
        <span className="text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">
          {m['label.pinned']()}
        </span>
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <button
                type="button"
                aria-label={m['label.pin_entries']()}
                title={m['label.pin_entries']()}
                disabled={
                  !ready ||
                  instances.isPending ||
                  servers.isPending ||
                  nothingToPin
                }
                className="text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
              >
                <PlusIcon className="size-3.5" />
              </button>
            }
          />
          <DropdownMenuContent align="end" className="w-52">
            {instanceList.length > 0 && (
              <DropdownMenuGroup>
                <DropdownMenuLabel>{m['nav.instances']()}</DropdownMenuLabel>
                {instanceList.map((instance) => (
                  <DropdownMenuCheckboxItem
                    key={instance.id}
                    checked={isPinned({ kind: 'instance', id: instance.id })}
                    onCheckedChange={() =>
                      toggle({ kind: 'instance', id: instance.id })
                    }
                  >
                    {instance.name}
                  </DropdownMenuCheckboxItem>
                ))}
              </DropdownMenuGroup>
            )}
            {serverList.length > 0 && (
              <DropdownMenuGroup>
                <DropdownMenuLabel>{m['nav.servers']()}</DropdownMenuLabel>
                {serverList.map((server) => (
                  <DropdownMenuCheckboxItem
                    key={server.id}
                    checked={isPinned({ kind: 'server', id: server.id })}
                    onCheckedChange={() =>
                      toggle({ kind: 'server', id: server.id })
                    }
                  >
                    {server.name}
                  </DropdownMenuCheckboxItem>
                ))}
              </DropdownMenuGroup>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      {pinned.length === 0 ? (
        <p className="px-3 py-1.5 text-[11px] text-muted-foreground/70">
          {m['label.nothing_pinned']()}
        </p>
      ) : (
        <div className="space-y-0.5">
          {rows.map((entry, index) => (
            <PinnedLink
              key={pinKey(entry)}
              entry={entry}
              pathname={pathname}
              onUnpin={() => toggle({ kind: entry.kind, id: entry.id })}
              drag={dragFor(index)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PinnedLink({
  entry,
  pathname,
  onUnpin,
  drag,
}: {
  entry: ResolvedPin;
  pathname: string;
  onUnpin: () => void;
  drag: DragProps;
}) {
  const active = pathname === `/${entry.kind}s/${entry.id}`;
  const content = <PinnedLinkContent entry={entry} onUnpin={onUnpin} />;
  const className = pinnedLinkClass(active, drag.dragging);

  const dragProps = {
    draggable: drag.draggable,
    onDragStart: (e: React.DragEvent) => {
      e.dataTransfer.effectAllowed = 'move';
      drag.onStart();
    },
    onDragEnter: drag.onEnter,
    onDragOver: (e: React.DragEvent) => e.preventDefault(),
    onDrop: (e: React.DragEvent) => {
      e.preventDefault();
      drag.onDrop();
    },
    onDragEnd: drag.onEnd,
  };

  if (entry.kind === 'server') {
    return (
      <Link
        to="/servers/$id"
        params={{ id: entry.id }}
        className={className}
        {...dragProps}
      >
        {content}
      </Link>
    );
  }

  return (
    <Link
      to="/instances/$id"
      params={{ id: entry.id }}
      className={className}
      {...dragProps}
    >
      {content}
    </Link>
  );
}

function pinnedLinkClass(active: boolean, dragging: boolean) {
  return cn(
    'group/pin flex items-center gap-2.5 px-3 py-1.5 transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
    active
      ? 'bg-muted text-foreground'
      : 'text-muted-foreground hover:bg-muted/60',
    dragging && 'opacity-50',
  );
}

function PinnedLinkContent({
  entry,
  onUnpin,
}: {
  entry: ResolvedPin;
  onUnpin: () => void;
}) {
  const Icon = entry.kind === 'server' ? HardDrivesIcon : CubeIcon;
  return (
    <>
      <span className="grid size-6 shrink-0 place-items-center overflow-hidden bg-muted ring-1 ring-border">
        {entry.iconUrl ? (
          <img src={entry.iconUrl} alt="" className="size-full object-cover" />
        ) : (
          <Icon className="size-3.5" />
        )}
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-xs text-foreground">
          {entry.name}
        </span>
        <span className="block truncate font-mono text-[10px] text-muted-foreground">
          {entry.flavor} · {entry.version}
        </span>
      </span>
      <span className="flex shrink-0 items-center gap-1.5 group-hover/pin:hidden group-focus-within/pin:hidden">
        {entry.running && entry.kind === 'server' && (
          <ServerPinPlayers id={entry.id} />
        )}
        {entry.running && entry.kind === 'instance' && entry.sessions > 1 && (
          <span
            className="font-mono text-[10px]"
            title={m['entry.sessions_running']({ count: entry.sessions })}
          >
            ×{entry.sessions}
          </span>
        )}
        {entry.running && <StatusDot tone="on" />}
      </span>
      <button
        type="button"
        aria-label={m['label.unpin']()}
        title={m['label.unpin']()}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onUnpin();
        }}
        className="hidden shrink-0 text-muted-foreground outline-none group-focus-within/pin:block group-hover/pin:block hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring"
      >
        <PushPinSlashIcon className="size-3.5" />
      </button>
    </>
  );
}

/** A running server's live player count, polled only while the pin is mounted. */
function ServerPinPlayers({ id }: { id: string }) {
  const ping = useQuery(serverQueries.ping(id));
  if (!ping.data) return null;
  return (
    <span className="font-mono text-[10px]" title={m['label.players']()}>
      {ping.data.playersOnline}/{ping.data.playersMax}
    </span>
  );
}
