import { PlusIcon, StackIcon, TrashIcon, XIcon } from '@phosphor-icons/react';
import { useMutation, useQueries, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { toast } from 'sonner';

import type { ContentKind, ProfileEntry } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { KindChips } from '@/features/content/components/kind-chips';
import { ContentInstallModal, profileTarget } from '@/features/content/install';
import { kindInfo } from '@/features/content/lib/kinds';
import { profileFilterKinds } from '@/features/profiles/page';
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
 * (kind chips + rows + the install modal) for consistency. A reference renders
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
        <Empty>{m['profiles.missing']()}</Empty>
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
  const filtered = kind ? items.filter((i) => i.kind === kind) : items;

  const removeReference = (ref: string) =>
    edit.mutate(
      { name: profile.name, remove: [ref] },
      { onError: (error) => toast.error(error.message) },
    );

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={m['profiles.page_title']()}
        parentTo="/profiles"
        icon={StackIcon}
        name={profile.name}
        badges={
          <Badge variant="outline" className="font-mono">
            {m['profiles.entries_count']({ count: profile.entries.length })}
          </Badge>
        }
        actions={
          <ConfirmDialog
            trigger={
              <Button variant="outline" data-icon="inline-start">
                <TrashIcon />
                {m['action.remove']()}
              </Button>
            }
            title={m['profiles.remove_title']({ name: profile.name })}
            description={m['profiles.remove_description']()}
            destructive
            confirmLabel={m['action.remove']()}
            onConfirm={() =>
              remove.mutate(profile.name, {
                onSuccess: () => navigate({ to: '/profiles' }),
                onError: (error) => toast.error(error.message),
              })
            }
          />
        }
      />

      <div className="flex-1 p-5">
        <KindChips
          kinds={profileFilterKinds}
          kind={kind}
          onKindChange={onKindChange}
          count={(k) => items.filter((i) => i.kind === k).length}
          action={
            <Button
              size="sm"
              variant="outline"
              data-icon="inline-start"
              onClick={() => setAdding(true)}
            >
              <PlusIcon weight="bold" />
              {m['content.add']()}
            </Button>
          }
        />
        {filtered.length === 0 ? (
          <Empty>
            {kind
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
                      {m['label.latest']()}
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={m['action.remove']()}
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

      <ContentInstallModal
        entry={profileTarget(profile)}
        open={adding}
        onOpenChange={setAdding}
      />
    </div>
  );
}
