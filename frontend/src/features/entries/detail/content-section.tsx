import { ArrowsClockwiseIcon, TrashIcon } from '@phosphor-icons/react';
import { useMutation, useQueries, useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import type { ContentKind } from '@/api';
import { Empty } from '@/components/empty';
import { FilterMenu } from '@/components/filter-menu';
import { SearchInput } from '@/components/search-input';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { kindGroup } from '@/features/content/components/kind-filter';
import { kindInfo } from '@/features/content/lib/kinds';
import { m } from '@/paraglide/messages.js';
import { instanceMutations, instanceQueries } from '@/queries/instance';
import { useJobMutation } from '@/queries/jobs';
import { serverMutations, serverQueries } from '@/queries/server';

import {
  filterContent,
  installedRef,
  type ListResult,
  type RowHandlers,
  rowKey,
  type SectionProps,
  type UpdatesResult,
} from './content';
import { ContentListResult } from './content-list';

/**
 * The content tab body: the kind filter + the filtered installed list, wired
 * to the daemon. The two entry kinds now share one factory shape, so the
 * queries and mutation handlers are selected by kind without splitting the
 * component — hook order stays stable across a re-render either way.
 */
export function ContentSection({
  entry,
  kinds,
  kind,
  onKindChange,
  action,
}: SectionProps) {
  const { id } = entry;
  const isServer = entry.kind === 'server';
  const queries = isServer ? serverQueries : instanceQueries;
  const content = isServer
    ? serverMutations.content
    : instanceMutations.content;

  const lists = useQueries({
    queries: kinds.map((k) => queries.content(id, k)),
  });
  const updates = useQueries({
    queries: kinds.map((k) => queries.contentUpdates(id, k)),
  });
  // A datapack that names no world targets every world the instance has, so the
  // per-world rows need the entry's current list. A server has exactly one.
  const worlds = useQuery({
    ...instanceQueries.worlds(id),
    enabled: !isServer && kinds.includes('data_pack'),
  });

  const enable = useMutation(content.enable(id));
  const remove = useMutation(content.remove(id));
  const update = useJobMutation(content.update(id));
  const setVersion = useJobMutation(content.setVersion(id));
  const handlers: RowHandlers = {
    // An omitted `worlds` covers every world the item targets — how the wire
    // reads an empty scope.
    onEnable: (item, enabled, worlds) =>
      enable.mutate({
        kind: item.kind,
        item: installedRef(item),
        enabled,
        worlds,
      }),
    onRemove: (item, worlds) =>
      remove.mutate({
        kind: item.kind,
        item: installedRef(item),
        worlds,
      }),
    onUpdate: (item) =>
      update.mutate({ kind: item.kind, item: installedRef(item) }),
    onSetVersion: (item, version) =>
      setVersion.mutate({
        kind: item.kind,
        item: installedRef(item),
        version: version.id,
      }),
  };

  return (
    <ContentSectionView
      entry={entry}
      kinds={kinds}
      kind={kind}
      onKindChange={onKindChange}
      action={action}
      lists={lists}
      updates={updates}
      handlers={handlers}
      entryWorlds={(worlds.data ?? []).map((world) => world.folder)}
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
  entryWorlds,
}: SectionProps & {
  lists: ListResult[];
  updates: UpdatesResult[];
  handlers: RowHandlers;
  entryWorlds: string[];
}) {
  const items = lists.flatMap((q) => q.data?.items ?? []);
  const untracked = lists.flatMap((q) => q.data?.untracked ?? []);
  const updatable = new Set(
    updates.flatMap((q) =>
      (q.data ?? []).filter((u) => u.updatable).map((u) => u.filename),
    ),
  );
  const checking = updates.some((q) => q.isFetching);

  // null = not selecting; a set of row keys while the select mode is active.
  const [selected, setSelected] = useState<Set<string> | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [search, setSearch] = useState('');
  const filtered = filterContent(items, kind, search);

  // Narrowing hides rows a selection may still hold; clear it so a batch-remove
  // can never delete a row the user can no longer see.
  const changeKind = (next?: ContentKind) => {
    setSelected(null);
    onKindChange(next);
  };
  const changeSearch = (next: string) => {
    setSelected(null);
    setSearch(next);
  };

  return (
    <>
      <div className="mb-5 flex items-center gap-2">
        <SearchInput
          value={search}
          onChange={changeSearch}
          placeholder={m['app.search.content']()}
          className="w-56"
        />
        <FilterMenu
          groups={[
            kindGroup({
              kinds,
              kind,
              onKindChange: changeKind,
              count: (k) => items.filter((c) => c.kind === k).length,
            }),
          ]}
        />
        <div className="ml-auto">
          {selected ? (
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={() => setSelected(null)}
              >
                {m['app.action.cancel']()}
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
                    {m['app.action.select']()}
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
          )}
        </div>
      </div>
      {filtered.length === 0 && search.trim() ? (
        <Empty>{m['content.browse.nothing_matches']()}</Empty>
      ) : filtered.length === 0 && kind ? (
        <Empty>
          {m['content.none_of_kind']({
            kind: kindInfo[kind].label().toLowerCase(),
          })}
        </Empty>
      ) : (
        <ContentListResult
          entry={entry}
          items={filtered}
          updatable={updatable}
          handlers={handlers}
          entryWorlds={entryWorlds}
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
        title={m['content.remove.selected_title']()}
        description={m['content.remove.selected_description']({
          count: selected?.size ?? 0,
        })}
        destructive
        confirmLabel={m['app.action.remove']()}
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
