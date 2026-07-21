import type { Icon } from '@phosphor-icons/react';
import {
  CubeIcon,
  GearSixIcon,
  HardDrivesIcon,
  PackageIcon,
  StackIcon,
  StorefrontIcon,
  TShirtIcon,
} from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

export interface NavItem {
  to: string;
  label: () => string;
  icon: Icon;
  /** Path prefixes that also light this item (detail routes, etc.). */
  match: string[];
}

export const nav: NavItem[] = [
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
  { to: '/skins', label: m['nav.skins'], icon: TShirtIcon, match: ['/skins'] },
];

export const settingsItem: NavItem = {
  to: '/settings',
  label: m['nav.settings'],
  icon: GearSixIcon,
  match: ['/settings'],
};

export function isActive(
  pathname: string,
  item: { to: string; match: string[] },
) {
  if (pathname === item.to) return true;
  return item.match.some(
    (prefix) => prefix !== '/' && pathname.startsWith(prefix),
  );
}

export function NavLink({ item, active }: { item: NavItem; active: boolean }) {
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
