import type { Icon } from '@phosphor-icons/react';
import {
  CaretUpDownIcon,
  CubeIcon,
  GearSixIcon,
  HardDrivesIcon,
  PackageIcon,
  PlusIcon,
  SignOutIcon,
  StorefrontIcon,
  TShirtIcon,
} from '@phosphor-icons/react';
import { Link, useLocation } from '@tanstack/react-router';
import { useState } from 'react';

import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { StatusDot } from '@/components/ui/status-dot';
import { pinnedInstances } from '@/features/entries/mock';
import { account, accounts } from '@/lib/mock';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

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
  return (
    <div className="border-t border-border p-2">
      <div className="flex items-center justify-between px-3 pt-1 pb-1.5">
        <span className="text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">
          {m['label.pinned']()}
        </span>
        <button
          type="button"
          aria-label={m['instances.new']()}
          className="text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring"
        >
          <PlusIcon className="size-3.5" />
        </button>
      </div>
      <div className="space-y-0.5">
        {pinnedInstances.map((i) => {
          const active = pathname === `/instances/${i.id}`;
          return (
            <Link
              key={i.id}
              to="/instances/$id"
              params={{ id: i.id }}
              className={cn(
                'flex items-center gap-2.5 px-3 py-1.5 transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
                active
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:bg-muted/60',
              )}
            >
              <span className="grid size-6 shrink-0 place-items-center bg-muted ring-1 ring-border">
                <CubeIcon className="size-3.5" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-xs text-foreground">
                  {i.name}
                </span>
                <span className="block truncate font-mono text-[10px] text-muted-foreground">
                  {i.flavor} · {i.game_version}
                </span>
              </span>
              {i.running && <StatusDot tone="on" />}
            </Link>
          );
        })}
      </div>
    </div>
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
  const others = accounts.filter((a) => !a.active);
  const [signingOut, setSigningOut] = useState(false);
  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <button
              type="button"
              className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors outline-none hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset aria-expanded:bg-muted"
            >
              <span className="grid size-7 shrink-0 place-items-center bg-muted text-[11px] font-semibold text-muted-foreground ring-1 ring-border">
                {account.name.slice(0, 2).toUpperCase()}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm">{account.name}</span>
                <span className="block truncate text-[11px] text-muted-foreground">
                  {m['account.microsoft']()}
                </span>
              </span>
              <CaretUpDownIcon className="size-4 shrink-0 text-muted-foreground" />
            </button>
          }
        />
        <DropdownMenuContent side="top" align="start" className="w-48">
          <DropdownMenuGroup>
            <DropdownMenuLabel>
              {m['account.signed_in_as']({ name: account.name })}
            </DropdownMenuLabel>
            {others.map((a) => (
              <DropdownMenuItem key={a.uuid}>
                <span className="grid size-5 shrink-0 place-items-center bg-muted text-[9px] font-semibold text-muted-foreground ring-1 ring-border">
                  {a.name.slice(0, 2).toUpperCase()}
                </span>
                {m['account.switch_to']({ name: a.name })}
              </DropdownMenuItem>
            ))}
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuItem>
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
        title={m['account.sign_out_title']({ name: account.name })}
        description={m['account.sign_out_description']()}
        destructive
        confirmLabel={m['account.sign_out']()}
        onConfirm={() => setSigningOut(false)}
      />
    </>
  );
}
