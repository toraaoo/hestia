import { PlusIcon, StackIcon, TrashIcon } from '@phosphor-icons/react';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import type { ContentKind } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  ContentInstallModal,
  profileTarget,
} from '@/features/content/install-modal';
import { KindChips } from '@/features/content/kind-chips';
import { kindInfo } from '@/features/content/kinds';
import { getProject } from '@/features/content/mock';
import { globalProfiles } from '@/features/profiles/mock';
import { profileFilterKinds } from '@/features/profiles/profiles-page';
import { m } from '@/paraglide/messages.js';

/** A profile reference as rendered on the detail page (mock, presentational). */
interface Reference {
  slug: string;
  name: string;
  kind: ContentKind;
  source: string;
}

/**
 * A global profile's detail page — the same shape as an entry's content tab
 * (`ContentSection` + the install modal) for consistency. A reference renders
 * as a content row pinned to "latest": the profile stores references, never
 * jars, so each apply resolves the version per instance. Local state over the
 * mock — nothing talks to a backend.
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
  const [adding, setAdding] = useState(false);

  if (!profile) {
    return (
      <div className="p-6">
        <Empty>{m['profiles.missing']()}</Empty>
      </div>
    );
  }

  const items: Reference[] = profile.entries.map((entry) => {
    const project = getProject(entry.slug);
    return {
      slug: entry.slug,
      name: project?.title ?? entry.slug,
      kind: project?.kind ?? 'mod',
      source: entry.source,
    };
  });
  const filtered = kind ? items.filter((i) => i.kind === kind) : items;

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
                  key={ref.slug}
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
