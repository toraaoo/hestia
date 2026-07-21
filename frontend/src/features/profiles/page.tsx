import { CaretRightIcon, PlusIcon, StackIcon } from '@phosphor-icons/react';
import { Link, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { toast } from 'sonner';
import type { ContentKind, GlobalProfile } from '@/api';
import { useSearch } from '@/components/app-shell/search-context';
import { Empty } from '@/components/empty';
import { Page } from '@/components/page';
import { CardGridSkeleton } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
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
import { type View, ViewToggle } from '@/features/entries/collection';
import { m } from '@/paraglide/messages.js';
import { useCreateGlobalProfile, useGlobalProfiles } from '@/queries/profile';

/** The kinds a global profile can reference — the selectable pool kinds. */
export const profileFilterKinds: ContentKind[] = [
  'mod',
  'resource_pack',
  'shader',
];

/**
 * The global profiles list: a grid/list of cards searchable by name, each
 * opening its detail page.
 */
export function ProfilesPage({
  view,
  onViewChange,
}: {
  view: View;
  onViewChange: (view: View) => void;
}) {
  const { query } = useSearch();
  const navigate = useNavigate();
  const list = useGlobalProfiles();
  const create = useCreateGlobalProfile();
  const [creating, setCreating] = useState(false);

  const profiles = list.data ?? [];
  const q = query.trim().toLowerCase();
  const filtered = profiles.filter((p) => !q || p.name.includes(q));

  return (
    <Page
      title={m['profiles.page_title']()}
      subtitle={m['profiles.page_description']()}
      loading={list.isPending}
      skeleton={
        <CardGridSkeleton
          grid="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3"
          count={6}
          card="h-24"
        />
      }
      search
      searchPlaceholder={m['profiles.search_placeholder']()}
      actions={
        <>
          <ViewToggle view={view} onView={onViewChange} />
          <Button
            size="sm"
            data-icon="inline-start"
            onClick={() => setCreating(true)}
          >
            <PlusIcon weight="bold" />
            {m['profiles.new_global']()}
          </Button>
        </>
      }
    >
      {filtered.length === 0 ? (
        <Empty>
          {profiles.length === 0
            ? m['profiles.global_empty']()
            : m['profiles.none_match']()}
        </Empty>
      ) : view === 'grid' ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
          {filtered.map((profile) => (
            <ProfileCard key={profile.name} profile={profile} />
          ))}
        </div>
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
        pending={create.isPending}
        onCreate={(name) =>
          create.mutate(name, {
            onSuccess: (profile) => {
              setCreating(false);
              navigate({
                to: '/profiles/$name',
                params: { name: profile.name },
              });
            },
            onError: (error) => toast.error(error.message),
          })
        }
      />
    </Page>
  );
}

function entrySummary(profile: GlobalProfile): string {
  return (
    profile.entries
      .slice(0, 3)
      .map((e) => e.slug || e.projectId)
      .join(' · ') || m['profiles.no_entries']()
  );
}

/** Grid tile mirroring `EntryCard`: art banner + name + chip + footer. */
function ProfileCard({ profile }: { profile: GlobalProfile }) {
  return (
    <Link
      to="/profiles/$name"
      params={{ name: profile.name }}
      className="group block outline-none focus-visible:ring-1 focus-visible:ring-ring"
    >
      <Card className="gap-0 overflow-hidden py-0 transition-colors group-hover:border-ember/40">
        <div className="relative flex h-24 items-center justify-center border-b border-border bg-muted/40">
          <StackIcon className="size-9 text-muted-foreground/40" />
          <span className="absolute top-2 left-2 bg-background/60 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground backdrop-blur-xs">
            {m['profiles.entries_count']({ count: profile.entries.length })}
          </span>
        </div>

        <div className="space-y-2 p-3">
          <div className="truncate text-sm font-medium">{profile.name}</div>
          <div className="truncate font-mono text-[11px] text-muted-foreground">
            {entrySummary(profile)}
          </div>
        </div>
      </Card>
    </Link>
  );
}

/** List row mirroring `EntryRow`: icon tile + name + summary + count. */
function ProfileRow({ profile }: { profile: GlobalProfile }) {
  return (
    <Link
      to="/profiles/$name"
      params={{ name: profile.name }}
      className="flex items-center gap-3 px-3 py-2.5 transition-colors outline-none hover:bg-muted/40 focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset"
    >
      <span className="grid size-9 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
        <StackIcon className="size-4.5" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{profile.name}</div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {entrySummary(profile)}
        </div>
      </div>
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
  pending,
  onCreate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taken: string[];
  pending: boolean;
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
          <Button disabled={invalid || pending} onClick={() => onCreate(slug)}>
            {m['action.confirm']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
