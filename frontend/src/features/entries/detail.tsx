import { DotsThreeIcon, TrashIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { type ReactNode, useState } from 'react';

import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { KindChips } from '@/features/content/kind-chips';
import { kindInfo } from '@/features/content/kinds';
import { getProject } from '@/features/content/mock';
import type { Backup, InstalledContent } from '@/features/entries/mock';
import { agoLabel, bytes } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

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
  const [list, setList] = useState(items);
  const remove = (item: InstalledContent) =>
    setList((l) => l.filter((c) => c.id !== item.id));

  const filtered = kind ? list.filter((c) => c.kind === kind) : list;

  return (
    <>
      <KindChips
        kinds={kinds}
        kind={kind}
        onKindChange={onKindChange}
        count={(k) => list.filter((c) => c.kind === k).length}
        action={action}
      />
      {filtered.length === 0 && kind ? (
        <Empty>
          {m['content.none_of_kind']({
            kind: kindInfo[kind].label().toLowerCase(),
          })}
        </Empty>
      ) : (
        <ContentList items={filtered} onRemove={remove} />
      )}
    </>
  );
}

export function ContentList({
  items,
  onRemove,
}: {
  items: InstalledContent[];
  onRemove?: (item: InstalledContent) => void;
}) {
  if (items.length === 0) {
    return <Empty>{m['content.none_installed']()}</Empty>;
  }
  return (
    <div className="divide-y divide-border border border-border">
      {items.map((c) => {
        const Icon = contentIcon(c.kind);
        // A local-file import has no project page to open.
        const linked = c.source !== 'file' && getProject(c.id) !== undefined;
        const body = (
          <>
            <Icon className="size-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm group-hover/item:underline group-hover/item:underline-offset-2">
                  {c.name}
                </span>
                {!c.enabled && (
                  <Badge variant="outline" className="shrink-0">
                    {m['content.disabled']()}
                  </Badge>
                )}
                {c.origin && (
                  <Badge variant="outline" className="shrink-0 font-mono">
                    {m['profiles.origin_badge']({ name: c.origin })}
                  </Badge>
                )}
                {c.updatable && (
                  <Badge className="shrink-0 bg-ember text-ember-foreground">
                    {m['content.update']()}
                  </Badge>
                )}
              </div>
              <div className="truncate font-mono text-[11px] text-muted-foreground">
                {contentKindLabel[c.kind]()} · {c.source} · {c.version}
              </div>
            </div>
          </>
        );
        return (
          <div key={c.id} className="flex items-center gap-3 px-3 py-2.5">
            {linked ? (
              <Link
                to="/browse/$kind/$id"
                params={{ kind: kindInfo[c.kind].slug, id: c.id }}
                className="group/item flex min-w-0 flex-1 items-center gap-3 outline-none focus-visible:ring-1 focus-visible:ring-ring"
              >
                {body}
              </Link>
            ) : (
              <div className="flex min-w-0 flex-1 items-center gap-3">
                {body}
              </div>
            )}
            <ConfirmDialog
              trigger={
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={m['action.remove']()}
                >
                  <TrashIcon className="size-4" />
                </Button>
              }
              title={m['content.remove_title']()}
              description={m['content.remove_description']({ name: c.name })}
              destructive
              confirmLabel={m['action.remove']()}
              onConfirm={() => onRemove?.(c)}
            />
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={m['action.more']()}
            >
              <DotsThreeIcon weight="bold" className="size-4" />
            </Button>
          </div>
        );
      })}
    </div>
  );
}

function backupKindLabel(kind: Backup['kind']): string {
  switch (kind) {
    case 'manual':
      return m['backup.kind_manual']();
    case 'scheduled':
      return m['backup.kind_scheduled']();
    case 'update':
      return m['backup.kind_update']();
  }
}

export function BackupList({ backups }: { backups: Backup[] }) {
  const [list, setList] = useState(backups);

  if (list.length === 0) {
    return <Empty>{m['backup.none']()}</Empty>;
  }
  return (
    <div className="divide-y divide-border border border-border">
      {list.map((b) => (
        <div key={b.id} className="flex items-center gap-3 px-3 py-2.5">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="text-sm">{agoLabel(b.created_unix)}</span>
              <Badge variant="secondary" className="shrink-0 capitalize">
                {backupKindLabel(b.kind)}
              </Badge>
            </div>
            <div className="font-mono text-[11px] text-muted-foreground">
              {bytes(b.size_bytes)}
            </div>
          </div>
          <ConfirmDialog
            trigger={
              <Button variant="outline" size="sm">
                {m['action.restore']()}
              </Button>
            }
            title={m['backup.restore_title']()}
            description={m['backup.restore_description']({
              when: agoLabel(b.created_unix),
            })}
            confirmLabel={m['action.restore']()}
            onConfirm={() => {}}
          />
          <ConfirmDialog
            trigger={
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={m['backup.delete_aria']()}
              >
                <TrashIcon className="size-4" />
              </Button>
            }
            title={m['backup.delete_title']()}
            description={m['backup.delete_description']({
              when: agoLabel(b.created_unix),
            })}
            destructive
            confirmLabel={m['action.delete']()}
            onConfirm={() => setList((l) => l.filter((x) => x.id !== b.id))}
          />
        </div>
      ))}
    </div>
  );
}
