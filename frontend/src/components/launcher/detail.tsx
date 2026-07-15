import type { Icon } from '@phosphor-icons/react';
import {
  CaretRightIcon,
  DotsThreeIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import type { ReactNode } from 'react';

import { contentIcon, contentKindLabel } from '@/components/launcher/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { agoLabel, bytes } from '@/lib/format';
import type { Backup, InstalledContent } from '@/lib/mock';

/** Breadcrumb + banner hero: parent link, big icon, name, badges, actions. */
export function DetailHero({
  parentLabel,
  parentTo,
  icon: Icon,
  name,
  badges,
  actions,
}: {
  parentLabel: string;
  parentTo: '/instances' | '/servers' | '/browse';
  icon: Icon;
  name: string;
  badges: ReactNode;
  actions: ReactNode;
}) {
  return (
    <div className="border-b border-border">
      <div className="flex items-center gap-1.5 px-5 py-2 text-xs text-muted-foreground">
        <Link to={parentTo} className="hover:text-foreground">
          {parentLabel}
        </Link>
        <CaretRightIcon className="size-3" />
        <span className="text-foreground">{name}</span>
      </div>

      <div className="flex items-end gap-4 bg-muted/25 px-5 pt-8 pb-5">
        <span className="grid size-16 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
          <Icon className="size-8" />
        </span>
        <div className="min-w-0 flex-1">
          <h1 className="truncate font-heading text-xl font-semibold">
            {name}
          </h1>
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            {badges}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">{actions}</div>
      </div>
    </div>
  );
}

/** A big number + label tile for an overview row. */
export function StatCard({
  value,
  label,
}: {
  value: ReactNode;
  label: string;
}) {
  return (
    <div className="border border-border px-4 py-3">
      <div className="font-heading text-xl font-semibold">{value}</div>
      <div className="mt-0.5 text-[11px] text-muted-foreground">{label}</div>
    </div>
  );
}

/** A titled side panel (Details, Quick actions). */
export function SideCard({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="border border-border">
      <div className="border-b border-border px-3 py-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {title}
      </div>
      <div className="p-3">{children}</div>
    </div>
  );
}

export function ContentList({ items }: { items: InstalledContent[] }) {
  if (items.length === 0) {
    return <Empty>No content installed yet. Add some from Browse.</Empty>;
  }
  return (
    <div className="divide-y divide-border border border-border">
      {items.map((c) => {
        const Icon = contentIcon(c.kind);
        return (
          <div key={c.id} className="flex items-center gap-3 px-3 py-2.5">
            <Icon className="size-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm">{c.name}</span>
                {!c.enabled && (
                  <Badge variant="outline" className="shrink-0">
                    Disabled
                  </Badge>
                )}
                {c.updatable && (
                  <Badge className="shrink-0 bg-ember text-ember-foreground">
                    Update
                  </Badge>
                )}
              </div>
              <div className="truncate font-mono text-[11px] text-muted-foreground">
                {contentKindLabel[c.kind]} · {c.source} · {c.version}
              </div>
            </div>
            <Button variant="ghost" size="icon-sm" aria-label="Remove">
              <TrashIcon className="size-4" />
            </Button>
            <Button variant="ghost" size="icon-sm" aria-label="More">
              <DotsThreeIcon weight="bold" className="size-4" />
            </Button>
          </div>
        );
      })}
    </div>
  );
}

export function BackupList({ backups }: { backups: Backup[] }) {
  if (backups.length === 0) {
    return <Empty>No backups yet. Create one to archive this world.</Empty>;
  }
  return (
    <div className="divide-y divide-border border border-border">
      {backups.map((b) => (
        <div key={b.id} className="flex items-center gap-3 px-3 py-2.5">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="text-sm">{agoLabel(b.created_unix)}</span>
              <Badge variant="secondary" className="shrink-0 capitalize">
                {b.kind}
              </Badge>
            </div>
            <div className="font-mono text-[11px] text-muted-foreground">
              {bytes(b.size_bytes)}
            </div>
          </div>
          <Button variant="outline" size="sm">
            Restore
          </Button>
          <Button variant="ghost" size="icon-sm" aria-label="Delete backup">
            <TrashIcon className="size-4" />
          </Button>
        </div>
      ))}
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return (
    <p className="border border-dashed border-border px-4 py-10 text-center text-xs text-muted-foreground">
      {children}
    </p>
  );
}
