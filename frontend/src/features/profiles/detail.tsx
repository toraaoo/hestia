import {
  MagnifyingGlassIcon,
  PlusIcon,
  PuzzlePieceIcon,
  StackIcon,
  TrashIcon,
  WarningCircleIcon,
  XIcon,
} from '@phosphor-icons/react';
import { useMutation, useQueries, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import type { ContentKind, ProfileEntry } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { FilterMenu } from '@/components/filter-menu';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { SearchInput } from '@/components/search-input';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  ContentInstallDialog,
  profileTarget,
} from '@/features/content/install';
import { profileFilterKinds } from '@/features/profiles/page';
import { kindGroup } from '@/features/shared/content/components';
import { kindInfo } from '@/features/shared/content/lib';
import { m } from '@/paraglide/messages.js';
import { contentQueries } from '@/queries/content';
import { profileMutations, profileQueries } from '@/queries/profile';

/** A profile reference joined with its resolved project detail. */
interface Reference {
  ref: string;
  name: string;
  kind: ContentKind;
  source: string;
}

const entryRef = (entry: ProfileEntry) => entry.slug || entry.projectId;

/**
 * A global profile's detail page — the same shape as an entry's content tab
 * (search + kind filter + rows + the install modal). A reference renders
 * as a content row pinned to "latest": the profile stores references, never
 * jars, so each apply resolves the version per instance. Titles and kinds come
 * from each reference's project detail, fetched per row.
 */
export function ProfileDetailPage({
  name,
  kind,
  onKindChange,
}: {
  name: string;
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
}) {
  const navigate = useNavigate();
  const list = useQuery(profileQueries.list());
  const remove = useMutation(profileMutations.remove());
  const edit = useMutation(profileMutations.edit());
  const [adding, setAdding] = useState(false);
  const [search, setSearch] = useState('');

  const profile = (list.data ?? []).find((p) => p.name === name);

  const projects = useQueries({
    queries: (profile?.entries ?? []).map((entry) =>
      contentQueries.project(entryRef(entry), entry.source),
    ),
  });

  if (list.isPending) {
    return (
      <div className="space-y-4 p-6">
        <Bone className="h-8 w-64" />
        <Bone className="h-40" />
      </div>
    );
  }

  if (!profile) {
    return (
      <div className="p-6">
        <Empty icon={WarningCircleIcon}>{m['profile.missing']()}</Empty>
      </div>
    );
  }

  const items: Reference[] = profile.entries.map((entry, index) => {
    const project = projects[index]?.data;
    return {
      ref: entryRef(entry),
      name: project?.title ?? entryRef(entry),
      kind: project?.kind ?? 'mod',
      source: entry.source,
    };
  });
  const q = search.trim().toLowerCase();
  const filtered = items.filter(
    (i) =>
      (!kind || i.kind === kind) && (!q || i.name.toLowerCase().includes(q)),
  );

  const removeReference = (ref: string) =>
    edit.mutate({ name: profile.name, remove: [ref] });

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={m['profile.global.title']()}
        parentTo="/profiles"
        icon={StackIcon}
        name={profile.name}
        badges={
          <Badge variant="outline" className="font-mono">
            {m['profile.global.entries_count']({
              count: profile.entries.length,
            })}
          </Badge>
        }
        actions={
          <ConfirmDialog
            trigger={
              <Button variant="outline" data-icon="inline-start">
                <TrashIcon />
                {m['app.action.remove']()}
              </Button>
            }
            title={m['profile.remove.title']({ name: profile.name })}
            description={m['profile.remove.description']()}
            destructive
            confirmLabel={m['app.action.remove']()}
            onConfirm={() =>
              remove.mutate(profile.name, {
                onSuccess: () => navigate({ to: '/profiles' }),
              })
            }
          />
        }
      />

      <div className="flex-1 p-5">
        <div className="mb-5 flex items-center gap-2">
          <SearchInput
            value={search}
            onChange={setSearch}
            placeholder={m['app.search.content']()}
            className="w-56"
          />
          <FilterMenu
            groups={[
              kindGroup({
                kinds: profileFilterKinds,
                kind,
                onKindChange,
                count: (k) => items.filter((i) => i.kind === k).length,
              }),
            ]}
          />
          <Button
            size="sm"
            variant="outline"
            data-icon="inline-start"
            className="ml-auto"
            onClick={() => setAdding(true)}
          >
            <PlusIcon weight="bold" />
            {m['content.add']()}
          </Button>
        </div>
        {filtered.length === 0 ? (
          <Empty
            icon={
              q
                ? MagnifyingGlassIcon
                : kind
                  ? contentIcon(kind)
                  : PuzzlePieceIcon
            }
          >
            {q
              ? m['content.browse.nothing_matches']()
              : kind
                ? m['content.none_of_kind']({
                    kind: kindInfo[kind].label().toLowerCase(),
                  })
                : m['content.none_installed']()}
          </Empty>
        ) : (
          <div className="divide-y divide-border border border-border">
            {filtered.map((ref) => {
              const Icon = contentIcon(ref.kind);
              return (
                <div
                  key={ref.ref}
                  className="flex items-center gap-3 px-3 py-2.5"
                >
                  <Icon className="size-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm">{ref.name}</div>
                    <div className="truncate font-mono text-[11px] text-muted-foreground">
                      {contentKindLabel[ref.kind]()} · {ref.source} ·{' '}
                      {m['app.label.latest']()}
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={m['app.action.remove']()}
                    disabled={edit.isPending}
                    onClick={() => removeReference(ref.ref)}
                  >
                    <XIcon className="size-4" />
                  </Button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <ContentInstallDialog
        entry={profileTarget(profile)}
        open={adding}
        onOpenChange={setAdding}
      />
    </div>
  );
}
