import {
  ArrowsClockwiseIcon,
  DotsThreeIcon,
  ProhibitIcon,
  SwapIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useQueries } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { type ReactNode, useState } from 'react';

import type { ContentKind, ContentVersion, InstalledContent } from '@/api';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
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
import { ChangeVersionModal } from '@/features/content/version-modal';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import {
  instanceQueries,
  useEnableInstanceContent,
  useRemoveInstanceContent,
  useSetInstanceContentVersion,
  useUpdateInstanceContent,
} from '@/queries/instance';
import {
  serverQueries,
  useEnableServerContent,
  useRemoveServerContent,
  useSetServerContentVersion,
  useUpdateServerContent,
} from '@/queries/server';

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

/** The entry a content tab acts on. */
export interface EntryTarget {
  kind: 'server' | 'instance';
  id: string;
  flavor: string;
  gameVersion: string;
}

/** How the daemon matches an item: its project id, else its filename. */
const installedRef = (i: InstalledContent) => i.projectId || i.filename;

/** A stable identity for one installed row (a datapack repeats per world). */
const rowKey = (i: InstalledContent) => `${i.kind}:${i.filename}:${i.world}`;

/** The world folder to narrow a datapack toggle/removal to (else none). */
const itemWorlds = (i: InstalledContent): string[] =>
  i.kind === 'data_pack' && i.world
    ? [i.world.split('/').pop() ?? i.world]
    : [];

/** The loader filter a kind's version lookup needs, given the entry's flavor. */
const kindLoader = (kind: ContentKind, flavor: string): string | undefined =>
  kind === 'mod' ? flavor : kind === 'data_pack' ? 'datapack' : undefined;

interface RowHandlers {
  onEnable: (item: InstalledContent, enabled: boolean) => void;
  onRemove: (item: InstalledContent) => void;
  onUpdate: (item: InstalledContent) => void;
  onSetVersion: (item: InstalledContent, version: ContentVersion) => void;
}

interface SectionProps {
  entry: EntryTarget;
  kinds: ContentKind[];
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
  action?: ReactNode;
}

/**
 * The content tab body: kind filter chips + the filtered installed list, wired
 * to the daemon. Dispatches to a server- or instance-bound section so the
 * mutation hooks stay unconditional.
 */
export function ContentSection(props: SectionProps) {
  return props.entry.kind === 'server' ? (
    <ServerContentSection {...props} />
  ) : (
    <InstanceContentSection {...props} />
  );
}

function ServerContentSection(props: SectionProps) {
  const { id } = props.entry;
  const lists = useQueries({
    queries: props.kinds.map((k) => serverQueries.content(id, k)),
  });
  const updates = useQueries({
    queries: props.kinds.map((k) => serverQueries.contentUpdates(id, k)),
  });
  const enable = useEnableServerContent(id);
  const remove = useRemoveServerContent(id);
  const update = useUpdateServerContent(id);
  const setVersion = useSetServerContentVersion(id);
  return (
    <ContentSectionView
      {...props}
      lists={lists}
      updates={updates}
      handlers={{
        onEnable: (item, enabled) =>
          enable.mutate({
            kind: item.kind,
            item: installedRef(item),
            enabled,
            worlds: itemWorlds(item),
          }),
        onRemove: (item) =>
          remove.mutate({
            kind: item.kind,
            item: installedRef(item),
            worlds: itemWorlds(item),
          }),
        onUpdate: (item) =>
          update.mutate({ kind: item.kind, item: installedRef(item) }),
        onSetVersion: (item, version) =>
          setVersion.mutate({
            kind: item.kind,
            item: installedRef(item),
            version: version.id,
          }),
      }}
    />
  );
}

function InstanceContentSection(props: SectionProps) {
  const { id } = props.entry;
  const lists = useQueries({
    queries: props.kinds.map((k) => instanceQueries.content(id, k)),
  });
  const updates = useQueries({
    queries: props.kinds.map((k) => instanceQueries.contentUpdates(id, k)),
  });
  const enable = useEnableInstanceContent(id);
  const remove = useRemoveInstanceContent(id);
  const update = useUpdateInstanceContent(id);
  const setVersion = useSetInstanceContentVersion(id);
  return (
    <ContentSectionView
      {...props}
      lists={lists}
      updates={updates}
      handlers={{
        onEnable: (item, enabled) =>
          enable.mutate({
            kind: item.kind,
            item: installedRef(item),
            enabled,
            worlds: itemWorlds(item),
          }),
        onRemove: (item) =>
          remove.mutate({
            kind: item.kind,
            item: installedRef(item),
            worlds: itemWorlds(item),
          }),
        onUpdate: (item) =>
          update.mutate({ kind: item.kind, item: installedRef(item) }),
        onSetVersion: (item, version) =>
          setVersion.mutate({
            kind: item.kind,
            item: installedRef(item),
            version: version.id,
          }),
      }}
    />
  );
}

type ListResult = { data?: { items: InstalledContent[]; untracked: string[] } };
type UpdatesResult = {
  data?: { filename: string; updatable: boolean }[];
  isFetching: boolean;
  refetch: () => void;
};

function ContentSectionView({
  entry,
  kinds,
  kind,
  onKindChange,
  action,
  lists,
  updates,
  handlers,
}: SectionProps & {
  lists: ListResult[];
  updates: UpdatesResult[];
  handlers: RowHandlers;
}) {
  const items = lists.flatMap((q) => q.data?.items ?? []);
  const untracked = lists.flatMap((q) => q.data?.untracked ?? []);
  const updatable = new Set(
    updates.flatMap((q) =>
      (q.data ?? []).filter((u) => u.updatable).map((u) => u.filename),
    ),
  );
  const checking = updates.some((q) => q.isFetching);
  const filtered = kind ? items.filter((c) => c.kind === kind) : items;

  // null = not selecting; a set of row keys while the select mode is active.
  const [selected, setSelected] = useState<Set<string> | null>(null);
  const [confirming, setConfirming] = useState(false);

  return (
    <>
      <KindChips
        kinds={kinds}
        kind={kind}
        onKindChange={onKindChange}
        count={(k) => items.filter((c) => c.kind === k).length}
        action={
          selected ? (
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={() => setSelected(null)}
              >
                {m['action.cancel']()}
              </Button>
              <Button
                size="sm"
                variant="destructive"
                data-icon="inline-start"
                disabled={selected.size === 0}
                onClick={() => setConfirming(true)}
              >
                <TrashIcon weight="bold" />
                {m['content.remove_count']({ count: selected.size })}
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              {items.length > 0 && (
                <>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setSelected(new Set())}
                  >
                    {m['content.select']()}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    data-icon="inline-start"
                    disabled={checking}
                    onClick={() => {
                      for (const q of updates) void q.refetch();
                    }}
                  >
                    <ArrowsClockwiseIcon weight="bold" />
                    {checking
                      ? m['content.checking_updates']()
                      : m['content.check_updates']()}
                  </Button>
                </>
              )}
              {action}
            </div>
          )
        }
      />
      {filtered.length === 0 && kind ? (
        <Empty>
          {m['content.none_of_kind']({
            kind: kindInfo[kind].label().toLowerCase(),
          })}
        </Empty>
      ) : (
        <ContentList
          entry={entry}
          items={filtered}
          updatable={updatable}
          handlers={handlers}
          selected={selected}
          onToggleSelect={(key) =>
            setSelected((prev) => {
              const next = new Set(prev);
              if (next.has(key)) next.delete(key);
              else next.add(key);
              return next;
            })
          }
        />
      )}
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={m['content.remove_selected_title']()}
        description={m['content.remove_selected_description']({
          count: selected?.size ?? 0,
        })}
        destructive
        confirmLabel={m['action.remove']()}
        onConfirm={() => {
          setConfirming(false);
          for (const item of items) {
            if (selected?.has(rowKey(item))) handlers.onRemove(item);
          }
          setSelected(null);
        }}
      />
      {untracked.length > 0 && (
        <p className="mt-3 text-[11px] text-muted-foreground">
          {m['content.untracked_note']({
            count: untracked.length,
            files: untracked.join(', '),
          })}
        </p>
      )}
    </>
  );
}

function ContentList({
  entry,
  items,
  updatable,
  handlers,
  selected,
  onToggleSelect,
}: {
  entry: EntryTarget;
  items: InstalledContent[];
  updatable: Set<string>;
  handlers: RowHandlers;
  selected: Set<string> | null;
  onToggleSelect: (key: string) => void;
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
            key={rowKey(c)}
            item={c}
            updatable={updatable.has(c.filename)}
            handlers={handlers}
            onChangeVersion={() => setChanging(c)}
            checked={selected ? selected.has(rowKey(c)) : undefined}
            onToggle={() => onToggleSelect(rowKey(c))}
          />
        ))}
      </div>
      <ChangeVersionModal
        item={changing}
        loader={changing ? kindLoader(changing.kind, entry.flavor) : undefined}
        gameVersion={entry.gameVersion || undefined}
        onOpenChange={(open) => !open && setChanging(null)}
        onPick={(item, version) => handlers.onSetVersion(item, version)}
      />
    </>
  );
}

function ContentRow({
  item,
  updatable,
  handlers,
  onChangeVersion,
  checked,
  onToggle,
}: {
  item: InstalledContent;
  updatable: boolean;
  handlers: RowHandlers;
  onChangeVersion: () => void;
  /** Set while the batch-select mode is active; undefined otherwise. */
  checked?: boolean;
  onToggle: () => void;
}) {
  const [removing, setRemoving] = useState(false);
  const selecting = checked !== undefined;
  const Icon = contentIcon(item.kind);
  // A local-file import has no project page to open and no versions to move
  // between — its only action is enable/disable and removal.
  const platform = item.source !== 'file' && !!item.projectId;
  const body = (
    <>
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm group-hover/item:underline group-hover/item:underline-offset-2">
            {item.title}
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
          {contentKindLabel[item.kind]()} · {item.source} · {item.versionNumber}
        </div>
      </div>
    </>
  );

  if (selecting) {
    const id = `select-${rowKey(item)}`;
    return (
      <label
        htmlFor={id}
        className={cn(
          'flex cursor-pointer items-center gap-3 px-3 py-2.5 transition-colors hover:bg-muted/60',
          !item.enabled && 'opacity-60',
        )}
      >
        <Checkbox id={id} checked={checked} onCheckedChange={onToggle} />
        {body}
      </label>
    );
  }
  return (
    <div className={item.enabled ? undefined : 'opacity-60'}>
      <div className="flex items-center gap-3 px-3 py-2.5">
        {platform ? (
          <Link
            to="/browse/$kind/$id"
            params={{
              kind: kindInfo[item.kind].slug,
              id: item.slug || item.projectId,
            }}
            className="group/item flex min-w-0 flex-1 items-center gap-3 outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            {body}
          </Link>
        ) : (
          <div className="flex min-w-0 flex-1 items-center gap-3">{body}</div>
        )}

        {updatable && item.enabled && (
          <Button
            size="sm"
            variant="outline"
            data-icon="inline-start"
            onClick={() => handlers.onUpdate(item)}
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
                {updatable && (
                  <DropdownMenuItem onClick={() => handlers.onUpdate(item)}>
                    <ArrowsClockwiseIcon />
                    {m['content.update_to_latest']()}
                  </DropdownMenuItem>
                )}
                <DropdownMenuItem onClick={onChangeVersion}>
                  <SwapIcon />
                  {m['content.change_version']()}
                </DropdownMenuItem>
              </>
            )}
            <DropdownMenuItem
              onClick={() => handlers.onEnable(item, !item.enabled)}
            >
              <ProhibitIcon />
              {item.enabled ? m['content.disable']() : m['content.enable']()}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
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
          description={m['content.remove_description']({ name: item.title })}
          destructive
          confirmLabel={m['action.remove']()}
          onConfirm={() => {
            setRemoving(false);
            handlers.onRemove(item);
          }}
        />
      </div>
    </div>
  );
}
