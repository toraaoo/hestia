import {
  ArrowsClockwiseIcon,
  FileIcon,
  FolderOpenIcon,
  MagnifyingGlassIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useMutation, useQueries, useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import type { ContentKind, UntrackedFile } from '@/api';
import { errorMessage, system } from '@/api';
import { Empty } from '@/components/empty';
import { FilterMenu } from '@/components/filter-menu';
import { contentIcon } from '@/components/icons';
import { SearchInput } from '@/components/search-input';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { kindGroup } from '@/features/shared/content/components';
import { kindInfo } from '@/features/shared/content/lib';
import { m } from '@/paraglide/messages.js';
import { instanceMutations, instanceQueries } from '@/queries/instance';
import { useJobMutation } from '@/queries/jobs';
import { modpackQueries } from '@/queries/modpack';
import { serverMutations, serverQueries } from '@/queries/server';
import { type ContentContext, ContentCtx, useContent } from '../hooks';
import {
  filterContent,
  filterUntracked,
  installedRef,
  type ListResult,
  type RowHandlers,
  rowKey,
  type SectionProps,
  type UpdatesResult,
} from '../lib';
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
  // A pack tags its installs by project id, so its own record is what turns
  // that tag into a name the row can show.
  const pack = useQuery(modpackQueries.status(entry.kind, id));

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

  const context: ContentContext = {
    entry,
    handlers,
    packName: pack.data?.name ?? '',
    entryWorlds: (worlds.data ?? []).map((world) => world.folder),
  };

  return (
    <ContentCtx.Provider value={context}>
      <ContentSectionView
        kinds={kinds}
        kind={kind}
        onKindChange={onKindChange}
        action={action}
        lists={lists}
        updates={updates}
      />
    </ContentCtx.Provider>
  );
}

function ContentSectionView({
  kinds,
  kind,
  onKindChange,
  action,
  lists,
  updates,
}: Omit<SectionProps, 'entry'> & {
  lists: ListResult[];
  updates: UpdatesResult[];
}) {
  const { handlers } = useContent();
  const items = lists.flatMap((q) => q.data?.items ?? []);
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
  // A per-kind query carries its own untracked files, so the kind filter reads
  // off the query's position rather than the file itself.
  const untracked = filterUntracked(
    lists.flatMap((list, i) =>
      kind && kinds[i] !== kind ? [] : (list.data?.untracked ?? []),
    ),
    search,
  );

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
        <Empty className="flex-1" icon={MagnifyingGlassIcon}>
          {m['content.browse.nothing_matches']()}
        </Empty>
      ) : filtered.length === 0 && kind ? (
        <Empty className="flex-1" icon={contentIcon(kind)}>
          {m['content.none_of_kind']({
            kind: kindInfo[kind].label().toLowerCase(),
          })}
        </Empty>
      ) : (
        <ContentListResult
          items={filtered}
          updatable={updatable}
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
      <UntrackedFiles files={untracked} />
    </>
  );
}

/**
 * The files sitting in the game's load dirs that no install record claims —
 * dropped in by hand, so the launcher leaves them alone. The count is the whole
 * message until asked for more — a folder with hundreds of them must never push
 * the list off the page — and they open in place as rows like the installed
 * pool, inside a region that scrolls on its own. Each row opens where it lives,
 * the only thing the launcher can do with a file it does not manage.
 */
function UntrackedFiles({ files }: { files: UntrackedFile[] }) {
  const [expanded, setExpanded] = useState(false);
  if (files.length === 0) return null;
  const reveal = (path: string) => {
    system.revealPath(path).catch((error: Error) => {
      toast.error(errorMessage(error));
    });
  };
  return (
    <div className="mt-4 text-[11px] text-muted-foreground">
      {m['content.untracked.summary']({ count: files.length })}{' '}
      <button
        type="button"
        aria-controls="untracked-files"
        aria-expanded={expanded}
        onClick={() => setExpanded((prev) => !prev)}
        className="underline underline-offset-3 outline-none hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring"
      >
        {expanded
          ? m['content.untracked.show_less']()
          : m['content.untracked.show_more']()}
      </button>
      {expanded && (
        <ul
          id="untracked-files"
          className="mt-2 max-h-52 divide-y divide-border overflow-y-auto border border-border text-foreground"
        >
          {files.map((file) => (
            <li
              key={file.path}
              className="group/untracked flex items-center gap-3 px-3 py-2.5"
            >
              <span className="grid size-7 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
                <FileIcon className="size-4" />
              </span>
              <span className="min-w-0 flex-1 truncate text-sm">
                {file.name}
              </span>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={m['content.untracked.reveal']()}
                title={file.path}
                className="opacity-0 transition-opacity focus-visible:opacity-100 group-hover/untracked:opacity-100"
                onClick={() => reveal(file.path)}
              >
                <FolderOpenIcon />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
