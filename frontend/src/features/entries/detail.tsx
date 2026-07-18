import {
  ArrowsClockwiseIcon,
  DotsThreeIcon,
  SwapIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { type ReactNode, useState } from 'react';

import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { KindChips } from '@/features/content/kind-chips';
import { kindInfo } from '@/features/content/kinds';
import type { ContentVersion } from '@/features/content/mock';
import { getProject, projectVersions } from '@/features/content/mock';
import { ChangeVersionModal } from '@/features/content/version-modal';
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
  const setVersion = (item: InstalledContent, version: ContentVersion) =>
    setList((l) =>
      l.map((c) =>
        c.id === item.id
          ? {
              ...c,
              version: version.versionNumber,
              updatable: newestVersion(c)?.id !== version.id,
            }
          : c,
      ),
    );
  const update = (item: InstalledContent) => {
    const newest = newestVersion(item);
    if (newest) setVersion(item, newest);
  };

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
        <ContentList
          items={filtered}
          onRemove={remove}
          onUpdate={update}
          onSetVersion={setVersion}
        />
      )}
    </>
  );
}

function newestVersion(item: InstalledContent): ContentVersion | undefined {
  const project = getProject(item.id);
  return project ? projectVersions(project)[0] : undefined;
}

export function ContentList({
  items,
  onRemove,
  onUpdate,
  onSetVersion,
}: {
  items: InstalledContent[];
  onRemove?: (item: InstalledContent) => void;
  onUpdate?: (item: InstalledContent) => void;
  onSetVersion?: (item: InstalledContent, version: ContentVersion) => void;
}) {
  const [changing, setChanging] = useState<InstalledContent | null>(null);

  if (items.length === 0) {
    return <Empty>{m['content.none_installed']()}</Empty>;
  }
  return (
    <>
      <div className="divide-y divide-border border border-border">
        {items.map((c) => (
          <ContentRow
            key={c.id}
            item={c}
            onRemove={onRemove}
            onUpdate={onUpdate}
            onChangeVersion={() => setChanging(c)}
          />
        ))}
      </div>
      <ChangeVersionModal
        item={changing}
        onOpenChange={(open) => !open && setChanging(null)}
        onPick={(item, version) => onSetVersion?.(item, version)}
      />
    </>
  );
}

function ContentRow({
  item,
  onRemove,
  onUpdate,
  onChangeVersion,
}: {
  item: InstalledContent;
  onRemove?: (item: InstalledContent) => void;
  onUpdate?: (item: InstalledContent) => void;
  onChangeVersion: () => void;
}) {
  const [removing, setRemoving] = useState(false);
  const Icon = contentIcon(item.kind);
  // A local-file import has no project page to open and no versions to move
  // between — its only action is removal.
  const platform = item.source !== 'file' && getProject(item.id) !== undefined;
  const body = (
    <>
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm group-hover/item:underline group-hover/item:underline-offset-2">
            {item.name}
          </span>
          {!item.enabled && (
            <Badge variant="outline" className="shrink-0">
              {m['content.disabled']()}
            </Badge>
          )}
          {item.origin && (
            <Badge variant="outline" className="shrink-0 font-mono">
              {m['profiles.origin_badge']({ name: item.origin })}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {contentKindLabel[item.kind]()} · {item.source} · {item.version}
        </div>
      </div>
    </>
  );

  return (
    <div className="flex items-center gap-3 px-3 py-2.5">
      {platform ? (
        <Link
          to="/browse/$kind/$id"
          params={{ kind: kindInfo[item.kind].slug, id: item.id }}
          className="group/item flex min-w-0 flex-1 items-center gap-3 outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          {body}
        </Link>
      ) : (
        <div className="flex min-w-0 flex-1 items-center gap-3">{body}</div>
      )}

      {item.updatable && (
        <Button
          size="sm"
          variant="outline"
          data-icon="inline-start"
          onClick={() => onUpdate?.(item)}
        >
          <ArrowsClockwiseIcon weight="bold" />
          {m['content.update']()}
        </Button>
      )}
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={m['action.more']()}
            >
              <DotsThreeIcon weight="bold" className="size-4" />
            </Button>
          }
        />
        <DropdownMenuContent align="end" className="w-48">
          {platform && (
            <>
              {item.updatable && (
                <DropdownMenuItem onClick={() => onUpdate?.(item)}>
                  <ArrowsClockwiseIcon />
                  {m['content.update_to_latest']()}
                </DropdownMenuItem>
              )}
              <DropdownMenuItem onClick={onChangeVersion}>
                <SwapIcon />
                {m['content.change_version']()}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
            </>
          )}
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setRemoving(true)}
          >
            <TrashIcon />
            {m['action.remove']()}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <ConfirmDialog
        open={removing}
        onOpenChange={setRemoving}
        title={m['content.remove_title']()}
        description={m['content.remove_description']({ name: item.name })}
        destructive
        confirmLabel={m['action.remove']()}
        onConfirm={() => {
          setRemoving(false);
          onRemove?.(item);
        }}
      />
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
              <span className="text-sm">{agoLabel(b.createdUnix)}</span>
              <Badge variant="secondary" className="shrink-0 capitalize">
                {backupKindLabel(b.kind)}
              </Badge>
            </div>
            <div className="font-mono text-[11px] text-muted-foreground">
              {bytes(b.sizeBytes)}
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
              when: agoLabel(b.createdUnix),
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
              when: agoLabel(b.createdUnix),
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
