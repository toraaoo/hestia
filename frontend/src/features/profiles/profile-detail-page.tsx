import { StackIcon, TrashIcon, XIcon } from '@phosphor-icons/react';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { KindChips } from '@/features/content/kind-chips';
import { kindInfo } from '@/features/content/kinds';
import { getProject } from '@/features/content/mock';
import { ProjectSearch } from '@/features/content/project-search';
import { globalProfiles } from '@/features/profiles/mock';
import { profileFilterKinds } from '@/features/profiles/profiles-page';
import { compact } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

/**
 * A global profile's detail page — the same shape as an entry's content tab
 * (kind chips + content rows) for consistency, with a content search in the
 * action slot to add references. Local state over the mock — nothing talks to
 * a backend.
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
  const profile = globalProfiles.find((p) => p.name === name);
  const [entries, setEntries] = useState(profile?.entries ?? []);

  if (!profile) {
    return (
      <div className="p-6">
        <Empty>{m['profiles.missing']()}</Empty>
      </div>
    );
  }

  const sync = (next: typeof entries) => {
    profile.entries = next;
    setEntries(next);
  };

  const kindOf = (slug: string) => getProject(slug)?.kind ?? ('mod' as const);
  const filtered = kind
    ? entries.filter((e) => kindOf(e.slug) === kind)
    : entries;

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={m['profiles.page_title']()}
        parentTo="/profiles"
        icon={StackIcon}
        name={profile.name}
        badges={
          <Badge variant="outline" className="font-mono">
            {m['profiles.entries_count']({ count: entries.length })}
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
            onConfirm={() => {
              const at = globalProfiles.findIndex(
                (p) => p.name === profile.name,
              );
              if (at >= 0) globalProfiles.splice(at, 1);
              navigate({ to: '/profiles' });
            }}
          />
        }
      />

      <div className="flex-1 p-5">
        <KindChips
          kinds={profileFilterKinds}
          kind={kind}
          onKindChange={onKindChange}
          count={(k) => entries.filter((e) => kindOf(e.slug) === k).length}
          action={
            <ProjectSearch
              exclude={new Set(entries.map((e) => e.slug))}
              placeholder={m['profiles.add_reference_placeholder']()}
              className="w-64"
              onPick={(project) =>
                sync([
                  ...entries,
                  {
                    source: 'modrinth',
                    project_id: project.id,
                    slug: project.id,
                  },
                ])
              }
            />
          }
        />

        {filtered.length === 0 ? (
          <Empty>
            {entries.length === 0
              ? m['profiles.no_entries']()
              : m['content.none_of_kind']({
                  kind: kind ? kindInfo[kind].label().toLowerCase() : '',
                })}
          </Empty>
        ) : (
          <div className="divide-y divide-border border border-border">
            {filtered.map((entry) => {
              const project = getProject(entry.slug);
              const Icon = contentIcon(project?.kind ?? 'mod');
              return (
                <div
                  key={entry.slug}
                  className="flex items-center gap-3 px-3 py-2.5"
                >
                  <Icon className="size-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm">
                      {project?.title ?? entry.slug}
                    </div>
                    <div className="truncate font-mono text-[11px] text-muted-foreground">
                      {contentKindLabel[project?.kind ?? 'mod']()} ·{' '}
                      {entry.source}
                      {project ? ` · ${compact(project.downloads)}` : ''}
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={m['action.remove']()}
                    onClick={() =>
                      sync(entries.filter((e) => e.slug !== entry.slug))
                    }
                  >
                    <XIcon className="size-4" />
                  </Button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
