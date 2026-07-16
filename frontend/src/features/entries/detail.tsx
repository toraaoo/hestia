import { DotsThreeIcon, TrashIcon } from '@phosphor-icons/react';
import type { ReactNode } from 'react';

import { chipClass } from '@/components/chip';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { kindInfo } from '@/features/browse/kinds';
import type { Backup, InstalledContent } from '@/features/entries/mock';
import { agoLabel, bytes } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';

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

/** The content tab body: kind filter chips + the filtered install list. */
export function ContentSection({
  items,
  kinds,
  kind,
  onKindChange,
  action,
}: {
  items: InstalledContent[];
  kinds: ContentKind[];
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
  action?: ReactNode;
}) {
  const filtered = kind ? items.filter((c) => c.kind === kind) : items;
  const count = (k: ContentKind) => items.filter((c) => c.kind === k).length;

  return (
    <>
      <div className="mb-5 flex flex-wrap items-center gap-1.5">
        <button
          type="button"
          className={chipClass(!kind)}
          onClick={() => onKindChange(undefined)}
        >
          All
        </button>
        {kinds.map((k) => (
          <button
            key={k}
            type="button"
            className={chipClass(kind === k)}
            onClick={() => onKindChange(k)}
          >
            {kindInfo[k].label}
            <span className="ml-1.5 font-mono text-[10px] opacity-60">
              {count(k)}
            </span>
          </button>
        ))}
        {action && <div className="ml-auto">{action}</div>}
      </div>
      {filtered.length === 0 && kind ? (
        <Empty>No {kindInfo[kind].label.toLowerCase()} installed.</Empty>
      ) : (
        <ContentList items={filtered} />
      )}
    </>
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
