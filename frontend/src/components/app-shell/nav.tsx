import type { Icon } from '@phosphor-icons/react';
import {
  CubeIcon,
  GearSixIcon,
  HardDrivesIcon,
  NewspaperIcon,
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
  { to: '/', label: m['app.nav.library'], icon: PackageIcon, match: ['/'] },
  {
    to: '/browse',
    label: m['app.nav.browse'],
    icon: StorefrontIcon,
    match: ['/browse'],
  },
  {
    to: '/instances',
    label: m['app.nav.instances'],
    icon: CubeIcon,
    match: ['/instances'],
  },
  {
    to: '/servers',
    label: m['app.nav.servers'],
    icon: HardDrivesIcon,
    match: ['/servers'],
  },
  {
    to: '/profiles',
    label: m['app.nav.profiles'],
    icon: StackIcon,
    match: ['/profiles'],
  },
  {
    to: '/skins',
    label: m['app.nav.skins'],
    icon: TShirtIcon,
    match: ['/skins'],
  },
  {
    to: '/news',
    label: m['app.nav.news'],
    icon: NewspaperIcon,
    match: ['/news'],
  },
];

export const settingsItem: NavItem = {
  to: '/settings',
  label: m['app.nav.settings'],
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

export function NavLink({
  item,
  active,
  badge,
}: {
  item: NavItem;
  active: boolean;
  /** Unread count; rendered only when above zero. */
  badge?: number;
}) {
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
      {badge !== undefined && badge > 0 && (
        <span className="ml-auto min-w-4 rounded-full bg-ember px-1.5 py-0.5 text-center text-[0.65rem] leading-none font-semibold text-background">
          {badge > 9 ? '9+' : badge}
        </span>
      )}
    </Link>
  );
}
