import type { Icon } from '@phosphor-icons/react';
import {
  CaretUpDownIcon,
  CubeIcon,
  GearSixIcon,
  HardDrivesIcon,
  PackageIcon,
  PlusIcon,
  SignOutIcon,
  StackIcon,
  StorefrontIcon,
  TShirtIcon,
} from '@phosphor-icons/react';
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
import { usePrefs } from '@/queries/prefs';
import { useServers } from '@/queries/server';

const PINNED_ENTRIES_KEY = 'sidebar.pinned-entries';

/** Stable empty fallback so the parse memo doesn't churn when nothing is pinned. */
const NO_PINS: unknown[] = [];

type PinnedKind = 'instance' | 'server';
type PinnedEntry = { kind: PinnedKind; id: string };
type ResolvedPin = PinnedEntry & {
  name: string;
  flavor: string;
  version: string;
  running: boolean;
};

/** Validate the persisted blob, dropping malformed and duplicate entries. */
function parsePinnedEntries(value: unknown): PinnedEntry[] {
  if (!Array.isArray(value)) return [];

  const entries: PinnedEntry[] = [];
  const seen = new Set<string>();
  for (const item of value) {
    if (typeof item !== 'object' || item === null) continue;
    const { kind, id } = item as Record<string, unknown>;
    if ((kind !== 'instance' && kind !== 'server') || typeof id !== 'string') {
      continue;
    }
    if (id === '') continue;

    const key = `${kind}:${id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    entries.push({ kind, id });
  }
  return entries;
}

function pinKey(pin: PinnedEntry): string {
  return `${pin.kind}:${pin.id}`;
}

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
  const { get, set, ready } = usePrefs();

  const instanceList = instances.data ?? [];
  const serverList = servers.data ?? [];

  const raw = get<unknown>(PINNED_ENTRIES_KEY, NO_PINS);
  const pinnedEntries = useMemo(() => parsePinnedEntries(raw), [raw]);

  const pinned = useMemo<ResolvedPin[]>(
    () =>
      pinnedEntries.flatMap((pin) => {
        if (pin.kind === 'instance') {
          const entry = instanceList.find((i) => i.id === pin.id);
          if (!entry) return [];
          return [
            {
              ...pin,
              name: entry.name,
              flavor: entry.flavor,
              version: entry.gameVersion,
              running: (entry.sessions ?? []).some(
                (session) => session.state === 'running',
              ),
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
          },
        ];
      }),
    [pinnedEntries, instanceList, serverList],
  );

  // Persist the pruned list when a pinned entry is deleted elsewhere. Both
  // lists must be loaded first, or a still-fetching query reads as empty and
  // would wrongly drop live pins. The ref keeps the effect off `set`'s churn.
  const setRef = useRef(set);
  setRef.current = set;
  useEffect(() => {
    if (!ready || !instances.data || !servers.data) return;
    if (pinned.length === pinnedEntries.length) return;
    setRef.current(
      PINNED_ENTRIES_KEY,
      pinned.map(({ kind, id }) => ({ kind, id })),
    );
  }, [ready, instances.data, servers.data, pinned, pinnedEntries]);

  const isPinned = (pin: PinnedEntry) =>
    pinnedEntries.some((entry) => pinKey(entry) === pinKey(pin));

  const togglePin = (pin: PinnedEntry) => {
    set(
      PINNED_ENTRIES_KEY,
      isPinned(pin)
        ? pinnedEntries.filter((entry) => pinKey(entry) !== pinKey(pin))
        : [...pinnedEntries, pin],
    );
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
                      togglePin({ kind: 'instance', id: instance.id })
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
                      togglePin({ kind: 'server', id: server.id })
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
          {pinned.map((entry) => (
            <PinnedLink key={pinKey(entry)} entry={entry} pathname={pathname} />
          ))}
        </div>
      )}
    </div>
  );
}

function PinnedLink({
  entry,
  pathname,
}: {
  entry: ResolvedPin;
  pathname: string;
}) {
  const active = pathname === `/${entry.kind}s/${entry.id}`;
  const content = <PinnedLinkContent entry={entry} />;

  if (entry.kind === 'server') {
    return (
      <Link
        to="/servers/$id"
        params={{ id: entry.id }}
        className={pinnedLinkClass(active)}
      >
        {content}
      </Link>
    );
  }

  return (
    <Link
      to="/instances/$id"
      params={{ id: entry.id }}
      className={pinnedLinkClass(active)}
    >
      {content}
    </Link>
  );
}

function pinnedLinkClass(active: boolean) {
  return cn(
    'flex items-center gap-2.5 px-3 py-1.5 transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
    active
      ? 'bg-muted text-foreground'
      : 'text-muted-foreground hover:bg-muted/60',
  );
}

function PinnedLinkContent({ entry }: { entry: ResolvedPin }) {
  const Icon = entry.kind === 'server' ? HardDrivesIcon : CubeIcon;
  return (
    <>
      <span className="grid size-6 shrink-0 place-items-center bg-muted ring-1 ring-border">
        <Icon className="size-3.5" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-xs text-foreground">
          {entry.name}
        </span>
        <span className="block truncate font-mono text-[10px] text-muted-foreground">
          {entry.flavor} · {entry.version}
        </span>
      </span>
      {entry.running && <StatusDot tone="on" />}
    </>
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
