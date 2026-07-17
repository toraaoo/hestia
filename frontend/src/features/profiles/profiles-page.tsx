import { CaretRightIcon, PlusIcon, StackIcon } from '@phosphor-icons/react';
import { Link, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { Empty } from '@/components/empty';
import { contentIcon } from '@/components/icons';
import { Page } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { KindChips } from '@/features/content/kind-chips';
import { kindInfo } from '@/features/content/kinds';
import { getProject } from '@/features/content/mock';
import type { GlobalProfile } from '@/features/profiles/mock';
import { globalProfiles, profileKinds } from '@/features/profiles/mock';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

/** The kinds a global profile can reference — the selectable pool kinds. */
export const profileFilterKinds: ContentKind[] = [
  'mod',
  'resourcepack',
  'shader',
];

/**
 * The global profiles list: searchable, filterable by referenced kind, each
 * card opening its detail page. Local state over the mock — nothing talks to
 * a backend.
 */
export function ProfilesPage({
  kind,
  onKindChange,
}: {
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
}) {
  const { query } = useSearch();
  const navigate = useNavigate();
  const [profiles, setProfiles] = useState<GlobalProfile[]>(globalProfiles);
  const [creating, setCreating] = useState(false);

  const q = query.trim().toLowerCase();
  const filtered = profiles.filter(
    (p) =>
      (!q || p.name.includes(q) || p.entries.some((e) => e.slug.includes(q))) &&
      (!kind || profileKinds(p).includes(kind)),
  );

  return (
    <Page
      title={m['profiles.page_title']()}
      subtitle={m['profiles.page_description']()}
      search
      searchPlaceholder={m['profiles.search_placeholder']()}
      actions={
        <Button
          size="sm"
          data-icon="inline-start"
          onClick={() => setCreating(true)}
        >
          <PlusIcon weight="bold" />
          {m['profiles.new_global']()}
        </Button>
      }
    >
      <KindChips
        kinds={profileFilterKinds}
        kind={kind}
        onKindChange={onKindChange}
        count={(k) =>
          profiles.filter((p) => profileKinds(p).includes(k)).length
        }
      />

      {filtered.length === 0 ? (
        <Empty>
          {profiles.length === 0
            ? m['profiles.global_empty']()
            : m['profiles.none_match']()}
        </Empty>
      ) : (
        <div className="divide-y divide-border border border-border">
          {filtered.map((profile) => (
            <ProfileRow key={profile.name} profile={profile} />
          ))}
        </div>
      )}

      <CreateGlobalDialog
        open={creating}
        onOpenChange={setCreating}
        taken={profiles.map((p) => p.name)}
        onCreate={(name) => {
          const profile = { name, entries: [] };
          globalProfiles.push(profile);
          setProfiles([...globalProfiles]);
          navigate({ to: '/profiles/$name', params: { name } });
        }}
      />
    </Page>
  );
}

function ProfileRow({ profile }: { profile: GlobalProfile }) {
  const kinds = profileKinds(profile);
  return (
    <Link
      to="/profiles/$name"
      params={{ name: profile.name }}
      className="flex items-center gap-3 px-3 py-2.5 transition-colors outline-none hover:bg-muted/60 focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset"
    >
      <span className="grid size-9 shrink-0 place-items-center bg-muted ring-1 ring-border">
        <StackIcon className="size-4.5 text-muted-foreground" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">
          {profile.name}
        </span>
        <span className="block truncate font-mono text-[11px] text-muted-foreground">
          {profile.entries
            .slice(0, 4)
            .map((e) => getProject(e.slug)?.title ?? e.slug)
            .join(' · ') || m['profiles.no_entries']()}
        </span>
      </span>
      <span className="flex shrink-0 items-center gap-1.5">
        {kinds.map((k) => {
          const Icon = contentIcon(k);
          return (
            <span
              key={k}
              title={kindInfo[k].label()}
              className="grid size-6 place-items-center bg-muted text-muted-foreground ring-1 ring-border"
            >
              <Icon className="size-3.5" />
            </span>
          );
        })}
      </span>
      <Badge variant="outline" className="shrink-0 font-mono">
        {m['profiles.entries_count']({ count: profile.entries.length })}
      </Badge>
      <CaretRightIcon className="size-4 shrink-0 text-muted-foreground" />
    </Link>
  );
}

function CreateGlobalDialog({
  open,
  onOpenChange,
  taken,
  onCreate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taken: string[];
  onCreate: (name: string) => void;
}) {
  const [name, setName] = useState('');
  // Mirrors the daemon's rule: a global profile's name doubles as its
  // filename, so it is slugged.
  const slug = name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  const invalid =
    slug.length === 0 || taken.some((t) => t.toLowerCase() === slug);

  const close = (next: boolean) => {
    if (!next) setName('');
    onOpenChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{m['profiles.create_title']()}</DialogTitle>
          <DialogDescription>
            {m['profiles.page_description']()}
          </DialogDescription>
        </DialogHeader>
        <Field>
          <FieldLabel>{m['profiles.name_label']()}</FieldLabel>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
          />
          {slug && slug !== name.trim() && (
            <p className="text-[11px] text-muted-foreground">{slug}</p>
          )}
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={invalid}
            onClick={() => {
              onCreate(slug);
              close(false);
            }}
          >
            {m['action.confirm']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
