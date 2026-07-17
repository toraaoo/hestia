import {
  FunnelSimpleIcon,
  PlusIcon,
  RowsIcon,
  SquaresFourIcon,
} from '@phosphor-icons/react';
import { Fragment } from 'react';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  EntryCard,
  type EntryCardData,
  EntryRow,
} from '@/features/entries/entry-card';
import { instances, servers } from '@/features/entries/mock';
import { agoLabel } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

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
    ? m['entry.sessions_running']({ count: i.sessions })
    : m['entry.last_played_ago']({ when: agoLabel(i.last_played_unix) }),
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
    ? m['status.preparing_ellipsis']()
    : `:${s.port ?? '—'} · ${
        s.running
          ? m['entry.players_online']({ count: s.players })
          : m['status.stopped']()
      }`,
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

export type FlavorGroup = {
  label: string;
  flavors: string[];
  value: string;
  onChange: (flavor: string) => void;
};

/** All flavor filters merged into one funnel-icon dropdown, shared by list views. */
export function FilterMenu({ groups }: { groups: FlavorGroup[] }) {
  const filtered = groups.some((g) => g.value !== 'all');
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon"
            aria-label={m['collection.filter_by_flavor']()}
            className={cn(filtered ? 'text-ember' : 'text-muted-foreground')}
          >
            <FunnelSimpleIcon weight={filtered ? 'bold' : 'regular'} />
          </Button>
        }
      />
      <DropdownMenuContent align="end" className="w-44">
        {groups.map((group, i) => (
          <Fragment key={group.label}>
            {i > 0 && <DropdownMenuSeparator />}
            <DropdownMenuGroup>
              <DropdownMenuLabel>{group.label}</DropdownMenuLabel>
              <DropdownMenuRadioGroup
                value={group.value}
                onValueChange={(value) => group.onChange(String(value))}
              >
                <DropdownMenuRadioItem value="all">
                  {m['label.all']()}
                </DropdownMenuRadioItem>
                {group.flavors.map((f) => (
                  <DropdownMenuRadioItem
                    key={f}
                    value={f}
                    className="capitalize"
                  >
                    {f}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuGroup>
          </Fragment>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
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
  const label =
    next === 'list'
      ? m['collection.switch_to_list']()
      : m['collection.switch_to_grid']();
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={label}
      title={label}
      onClick={() => onView(next)}
      className="text-muted-foreground"
    >
      <Icon className="size-4" />
    </Button>
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
