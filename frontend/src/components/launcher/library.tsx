import { PlusIcon, RowsIcon, SquaresFourIcon } from '@phosphor-icons/react';

import {
  EntryCard,
  type EntryCardData,
  EntryRow,
} from '@/components/launcher/entry-card';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { agoLabel } from '@/lib/format';
import { instances, servers } from '@/lib/mock';
import { cn } from '@/lib/utils';

export type View = 'grid' | 'list';

export const instanceCards: EntryCardData[] = instances.map((i) => ({
  id: i.id,
  name: i.name,
  kind: 'instance',
  flavor: i.flavor,
  version: i.game_version,
  running: i.running,
  ready: true,
  subtitle: i.running
    ? `${i.sessions} running`
    : `Last played ${agoLabel(i.last_played_unix)}`,
}));

export const serverCards: EntryCardData[] = servers.map((s) => ({
  id: s.id,
  name: s.name,
  kind: 'server',
  flavor: s.flavor,
  version: s.game_version,
  running: s.running,
  ready: s.ready,
  subtitle: !s.ready
    ? 'Preparing…'
    : `:${s.port ?? '—'} · ${s.running ? `${s.players} online` : 'Stopped'}`,
}));

export const instanceFlavors = [...new Set(instanceCards.map((c) => c.flavor))];
export const serverFlavors = [...new Set(serverCards.map((c) => c.flavor))];

export function filterCards(
  cards: EntryCardData[],
  query: string,
  flavor: string = 'all',
) {
  const q = query.trim().toLowerCase();
  return cards.filter((c) => {
    if (flavor !== 'all' && c.flavor !== flavor) return false;
    if (!q) return true;
    return (
      c.name.toLowerCase().includes(q) ||
      c.flavor.toLowerCase().includes(q) ||
      c.version.toLowerCase().includes(q)
    );
  });
}

/** A single-select flavor filter (All + each flavor), shared by list views. */
export function FlavorFilter({
  value,
  onChange,
  flavors,
}: {
  value: string;
  onChange: (v: string) => void;
  flavors: string[];
}) {
  return (
    <ToggleGroup
      variant="outline"
      size="sm"
      value={[value]}
      onValueChange={(vals: string[]) => {
        const next = vals[vals.length - 1];
        if (next) onChange(next);
      }}
    >
      <ToggleGroupItem value="all">All</ToggleGroupItem>
      {flavors.map((f) => (
        <ToggleGroupItem key={f} value={f} className="capitalize">
          {f}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}

/** A single toggle that flips grid⇄list, showing the view you'll switch to. */
export function ViewToggle({
  view,
  onView,
}: {
  view: View;
  onView: (v: View) => void;
}) {
  const next: View = view === 'grid' ? 'list' : 'grid';
  const Icon = next === 'list' ? RowsIcon : SquaresFourIcon;
  return (
    <button
      type="button"
      aria-label={`Switch to ${next} view`}
      title={`Switch to ${next} view`}
      onClick={() => onView(next)}
      className={cn(
        'flex size-8 items-center justify-center border border-border text-muted-foreground transition-colors outline-none hover:bg-muted hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring',
      )}
    >
      <Icon className="size-4" />
    </button>
  );
}

export function EntryCollection({
  cards,
  view,
  empty,
}: {
  cards: EntryCardData[];
  view: View;
  empty: string;
}) {
  if (cards.length === 0) {
    return (
      <p className="border border-dashed border-border px-4 py-10 text-center text-xs text-muted-foreground">
        {empty}
      </p>
    );
  }
  if (view === 'list') {
    return (
      <div className="divide-y divide-border border border-border">
        {cards.map((entry) => (
          <EntryRow key={entry.id} entry={entry} />
        ))}
      </div>
    );
  }
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
      {cards.map((entry) => (
        <EntryCard key={entry.id} entry={entry} />
      ))}
    </div>
  );
}

/** The dashed "new entry" tile shown at the end of a grid. */
export function NewTile({ label }: { label: string }) {
  return (
    <button
      type="button"
      className="flex min-h-[11.5rem] flex-col items-center justify-center gap-2 border border-dashed border-border text-xs text-muted-foreground transition-colors outline-none hover:border-ember/40 hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring"
    >
      <PlusIcon className="size-6" />
      {label}
    </button>
  );
}
