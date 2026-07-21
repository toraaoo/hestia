import type { Icon } from '@phosphor-icons/react';
import {
  CaretUpDownIcon,
  CubeIcon,
  GearSixIcon,
  HardDrivesIcon,
  PackageIcon,
  PlusIcon,
  PushPinSlashIcon,
  SignOutIcon,
  StackIcon,
  StorefrontIcon,
  TShirtIcon,
} from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { Link, useLocation } from '@tanstack/react-router';
import { useEffect, useMemo, useRef, useState } from 'react';
import { AccountAvatar } from '@/components/app-shell/account-avatar';
import { Bone } from '@/components/skeleton';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { StatusDot } from '@/components/ui/status-dot';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries/accounts';
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

interface NavItem {
  to: string;
  label: () => string;
  icon: Icon;
  /** Path prefixes that also light this item (detail routes, etc.). */
  match: string[];
}

const nav: NavItem[] = [
  { to: '/', label: m['nav.library'], icon: PackageIcon, match: ['/'] },
  {
    to: '/browse',
    label: m['nav.browse'],
    icon: StorefrontIcon,
    match: ['/browse'],
  },
  {
    to: '/instances',
    label: m['nav.instances'],
    icon: CubeIcon,
    match: ['/instances'],
  },
  {
    to: '/servers',
    label: m['nav.servers'],
    icon: HardDrivesIcon,
    match: ['/servers'],
  },
  {
    to: '/profiles',
    label: m['profiles.nav'],
    icon: StackIcon,
    match: ['/profiles'],
  },
  {
    to: '/skins',
    label: m['nav.skins'],
    icon: TShirtIcon,
    match: ['/skins'],
  },
];

function isActive(pathname: string, item: { to: string; match: string[] }) {
  if (pathname === item.to) return true;
  return item.match.some((m) => m !== '/' && pathname.startsWith(m));
}

export function Sidebar() {
  const { pathname } = useLocation();

  return (
    <nav className="flex w-52 shrink-0 flex-col border-r border-border bg-sidebar">
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-0.5 p-2">
          {nav.map((item) => (
            <NavLink
              key={item.to}
              item={item}
              active={isActive(pathname, item)}
            />
          ))}
        </div>

        <PinnedSection pathname={pathname} />
      </div>

      <div className="flex h-[108px] flex-col gap-0.5 border-t border-border p-2">
        <NavLink
          item={{
            to: '/settings',
            label: m['nav.settings'],
            icon: GearSixIcon,
            match: ['/settings'],
          }}
          active={isActive(pathname, { to: '/settings', match: ['/settings'] })}
        />

        <AccountMenu />
      </div>
    </nav>
  );
}

function PinnedSection({ pathname }: { pathname: string }) {
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

  // Reorder previews locally while dragging; the drop commits it to prefs.
  const [drag, setDrag] = useState<{
    list: ResolvedPin[];
    index: number;
  } | null>(null);
  const rows = drag?.list ?? pinned;

  const beginDrag = (index: number) => setDrag({ list: pinned, index });
  const dragOver = (index: number) =>
    setDrag((current) => {
      if (!current || current.index === index) return current;
      const list = [...current.list];
      const [moved] = list.splice(current.index, 1);
      list.splice(index, 0, moved);
      return { list, index };
    });
  const endDrag = (commit: boolean) => {
    if (commit && drag) save(drag.list.map(({ kind, id }) => ({ kind, id })));
    setDrag(null);
  };

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
              draggable={rows.length > 1}
              dragging={drag !== null && drag.index === index}
              onDragStart={() => beginDrag(index)}
              onDragEnter={() => dragOver(index)}
              onDrop={() => endDrag(true)}
              onDragEnd={() => endDrag(false)}
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
  draggable,
  dragging,
  onDragStart,
  onDragEnter,
  onDrop,
  onDragEnd,
}: {
  entry: ResolvedPin;
  pathname: string;
  onUnpin: () => void;
  draggable: boolean;
  dragging: boolean;
  onDragStart: () => void;
  onDragEnter: () => void;
  onDrop: () => void;
  onDragEnd: () => void;
}) {
  const active = pathname === `/${entry.kind}s/${entry.id}`;
  const content = <PinnedLinkContent entry={entry} onUnpin={onUnpin} />;
  const className = pinnedLinkClass(active, dragging);

  const dragProps = {
    draggable,
    onDragStart: (e: React.DragEvent) => {
      e.dataTransfer.effectAllowed = 'move';
      onDragStart();
    },
    onDragEnter,
    onDragOver: (e: React.DragEvent) => e.preventDefault(),
    onDrop: (e: React.DragEvent) => {
      e.preventDefault();
      onDrop();
    },
    onDragEnd,
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

/** Server List Ping players, polled only while the pin is mounted running. */
function ServerPinPlayers({ id }: { id: string }) {
  const ping = useQuery(serverQueries.ping(id));
  if (!ping.data) return null;
  return (
    <span className="font-mono text-[10px]" title={m['label.players']()}>
      {ping.data.playersOnline}/{ping.data.playersMax}
    </span>
  );
}

function NavLink({ item, active }: { item: NavItem; active: boolean }) {
  const { icon: Icon, to, label } = item;
  return (
    <Link
      to={to}
      aria-current={active ? 'page' : undefined}
      className={cn(
        'relative flex items-center gap-2.5 px-3 py-2 text-sm transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
        active
          ? 'bg-muted font-medium text-foreground'
          : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
      )}
    >
      <span
        className={cn(
          'absolute top-1/2 left-0 h-5 w-0.5 -translate-y-1/2 bg-ember transition-opacity',
          active ? 'opacity-100' : 'opacity-0',
        )}
      />
      <Icon weight={active ? 'fill' : 'regular'} className="size-4.5" />
      {label()}
    </Link>
  );
}

function AccountMenu() {
  const {
    accounts,
    active,
    isPending,
    login,
    switch: switchAccount,
    remove: removeAccount,
  } = useAccounts();
  const [signingOut, setSigningOut] = useState(false);

  const others = active
    ? accounts.filter((a) => a.uuid !== active.uuid)
    : accounts;

  if (isPending) {
    return (
      <div className="flex w-full items-center gap-2.5 px-3 py-2">
        <Bone className="size-7 shrink-0" />
        <span className="min-w-0 flex-1 space-y-1.5">
          <Bone className="h-3.5 w-24" />
          <Bone className="h-2.5 w-16" />
        </span>
      </div>
    );
  }

  if (!active) {
    return (
      <button
        type="button"
        disabled={login.isPending}
        onClick={() => login.mutate()}
        className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors outline-none hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset disabled:opacity-60"
      >
        <span className="grid size-7 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
          <PlusIcon className="size-4" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm">
            {login.isPending
              ? m['account.signing_in']()
              : m['account.sign_in']()}
          </span>
          <span className="block truncate text-[11px] text-muted-foreground">
            {login.isError
              ? m['account.sign_in_failed']()
              : m['account.not_signed_in']()}
          </span>
        </span>
      </button>
    );
  }

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <button
              type="button"
              className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors outline-none hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset aria-expanded:bg-muted"
            >
              <AccountAvatar
                uuid={active.uuid}
                name={active.name}
                size={28}
                className="text-[11px]"
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm">{active.name}</span>
                <span className="block truncate text-[11px] text-muted-foreground">
                  {login.isPending
                    ? m['account.signing_in']()
                    : m['account.microsoft']()}
                </span>
              </span>
              <CaretUpDownIcon className="size-4 shrink-0 text-muted-foreground" />
            </button>
          }
        />
        <DropdownMenuContent side="top" align="start" className="w-48">
          <DropdownMenuGroup>
            <DropdownMenuLabel>
              {m['account.signed_in_as']({ name: active.name })}
            </DropdownMenuLabel>
            {others.map((a) => (
              <DropdownMenuItem
                key={a.uuid}
                onClick={() => switchAccount.mutate(a.uuid)}
              >
                <AccountAvatar
                  uuid={a.uuid}
                  name={a.name}
                  size={20}
                  className="text-[9px]"
                />
                {m['account.switch_to']({ name: a.name })}
              </DropdownMenuItem>
            ))}
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem
              disabled={login.isPending}
              onClick={() => login.mutate()}
            >
              <PlusIcon />
              {m['account.add']()}
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setSigningOut(true)}>
              <SignOutIcon />
              {m['account.sign_out']()}
            </DropdownMenuItem>
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={signingOut}
        onOpenChange={setSigningOut}
        title={m['account.sign_out_title']({ name: active.name })}
        description={m['account.sign_out_description']()}
        destructive
        confirmLabel={m['account.sign_out']()}
        onConfirm={() => {
          removeAccount.mutate(active.uuid);
          setSigningOut(false);
        }}
      />
    </>
  );
}
