import { ArrowsClockwiseIcon, TrashIcon } from '@phosphor-icons/react';
import { useQueries } from '@tanstack/react-query';
import { useState } from 'react';

import type { ContentVersion, InstalledContent } from '@/api';
import { Empty } from '@/components/empty';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { KindChips } from '@/features/content/kind-chips';
import { kindInfo } from '@/features/content/kinds';
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

import {
  installedRef,
  itemWorlds,
  type ListResult,
  type RowHandlers,
  rowKey,
  type SectionProps,
  type UpdatesResult,
} from './content';
import { ContentList } from './content-list';

/** One mutation from each content op, shared by both entry kinds. */
interface ContentMutations {
  enable: {
    mutate: (v: {
      kind: InstalledContent['kind'];
      item: string;
      enabled: boolean;
      worlds: string[];
    }) => void;
  };
  remove: {
    mutate: (v: {
      kind: InstalledContent['kind'];
      item: string;
      worlds: string[];
    }) => void;
  };
  update: {
    mutate: (v: { kind: InstalledContent['kind']; item: string }) => void;
  };
  setVersion: {
    mutate: (v: {
      kind: InstalledContent['kind'];
      item: string;
      version: string;
    }) => void;
  };
}

/** Map the entry's mutation hooks onto the row-level handler callbacks. */
function buildHandlers(m: ContentMutations): RowHandlers {
  return {
    onEnable: (item, enabled) =>
      m.enable.mutate({
        kind: item.kind,
        item: installedRef(item),
        enabled,
        worlds: itemWorlds(item),
      }),
    onRemove: (item) =>
      m.remove.mutate({
        kind: item.kind,
        item: installedRef(item),
        worlds: itemWorlds(item),
      }),
    onUpdate: (item) =>
      m.update.mutate({ kind: item.kind, item: installedRef(item) }),
    onSetVersion: (item, version: ContentVersion) =>
      m.setVersion.mutate({
        kind: item.kind,
        item: installedRef(item),
        version: version.id,
      }),
  };
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
  const handlers = buildHandlers({
    enable: useEnableServerContent(id),
    remove: useRemoveServerContent(id),
    update: useUpdateServerContent(id),
    setVersion: useSetServerContentVersion(id),
  });
  return (
    <ContentSectionView
      {...props}
      lists={lists}
      updates={updates}
      handlers={handlers}
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
  const handlers = buildHandlers({
    enable: useEnableInstanceContent(id),
    remove: useRemoveInstanceContent(id),
    update: useUpdateInstanceContent(id),
    setVersion: useSetInstanceContentVersion(id),
  });
  return (
    <ContentSectionView
      {...props}
      lists={lists}
      updates={updates}
      handlers={handlers}
    />
  );
}

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
