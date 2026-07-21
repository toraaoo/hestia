import {
  FunnelSimpleIcon,
  RowsIcon,
  SquaresFourIcon,
} from '@phosphor-icons/react';
import { Fragment } from 'react';

import type { InstanceInfo, ServerInfo } from '@/api';
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
  type EntryCardModel,
  EntryRow,
} from '@/features/entries/components/entry-card';
import { agoLabel } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

export type View = 'grid' | 'list';

/** Quick-action wiring shared by every card builder. */
export type CardActions = {
  busy: boolean;
  onStart: () => void;
  onStop: () => void;
};

function runningSessions(instance: InstanceInfo): number {
  return (instance.sessions ?? []).filter((s) => s.state === 'running').length;
}

/** Map a live server record to the card shape, with its quick actions wired. */
export function serverToCard(
  server: ServerInfo,
  actions: CardActions,
): EntryCardModel {
  const running = server.process?.state === 'running';
  const address = server.gamePort ? `:${server.gamePort}` : '';
  const state = running ? m['status.online']() : m['status.stopped']();
  return {
    id: server.id,
    name: server.name,
    kind: 'server',
    flavor: server.flavor,
    version: server.gameVersion,
    running,
    ready: server.ready,
    subtitle: !server.ready
      ? m['status.preparing_ellipsis']()
      : address
        ? `${address} · ${state}`
        : state,
    busy: actions.busy,
    onStart: actions.onStart,
    onStop: actions.onStop,
  };
}

/** Map a live instance record to the card shape, with its quick actions wired. */
export function instanceToCard(
  instance: InstanceInfo,
  actions: CardActions,
  lastPlayedUnix?: number,
): EntryCardModel {
  const running = runningSessions(instance);
  return {
    id: instance.id,
    name: instance.name,
    kind: 'instance',
    flavor: instance.flavor,
    version: instance.gameVersion,
    running: running > 0,
    ready: true,
    subtitle:
      running > 0
        ? m['entry.sessions_running']({ count: running })
        : lastPlayedUnix
          ? `${m['label.last_played']()} ${agoLabel(lastPlayedUnix)}`
          : m['status.stopped'](),
    busy: actions.busy,
    onStart: actions.onStart,
    onStop: actions.onStop,
  };
}

export function flavorsOf(cards: EntryCardModel[]): string[] {
  return [...new Set(cards.map((c) => c.flavor))];
}

export function filterCards(
  cards: EntryCardModel[],
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
  cards: EntryCardModel[];
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
