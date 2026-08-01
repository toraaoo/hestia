import { RowsIcon, SquaresFourIcon } from '@phosphor-icons/react';

import type { InstanceInfo, ServerInfo } from '@/api';
import type { FilterGroup } from '@/components/filter-menu';
import { Button } from '@/components/ui/button';
import {
  EntryCard,
  type EntryCardModel,
  EntryRow,
} from '@/features/shared/entry/components';
import { agoLabel } from '@/lib/format';
import { runningSessions } from '@/lib/sessions';
import { m } from '@/paraglide/messages.js';

export type View = 'grid' | 'list';

/** Quick-action wiring shared by every card builder. */
export type CardActions = {
  busy: boolean;
  onStart: () => void;
  /** `session` names one session to stop; absent stops the entry outright. */
  onStop: (session?: string) => void;
  /** Instances only: launch alongside the sessions already running. */
  onNewSession?: () => void;
  launching?: boolean;
  stopping?: boolean;
};

/** Map a live server record to the card shape, with its quick actions wired. */
export function serverToCard(
  server: ServerInfo,
  actions: CardActions,
): EntryCardModel {
  const running = server.process?.state === 'running';
  const address = server.gamePort ? `:${server.gamePort}` : '';
  const state = running ? m['app.status.online']() : m['app.status.stopped']();
  return {
    id: server.id,
    name: server.name,
    kind: 'server',
    flavor: server.flavor,
    version: server.gameVersion,
    running,
    ready: server.ready,
    subtitle: !server.ready
      ? m['app.status.preparing_ellipsis']()
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
  const sessions = runningSessions(instance);
  return {
    id: instance.id,
    name: instance.name,
    kind: 'instance',
    flavor: instance.flavor,
    version: instance.gameVersion,
    running: sessions.length > 0,
    ready: true,
    subtitle:
      sessions.length > 0
        ? m['entry.sessions_running']({ count: sessions.length })
        : lastPlayedUnix
          ? `${m['app.label.last_played']()} ${agoLabel(lastPlayedUnix)}`
          : m['app.status.stopped'](),
    sessions,
    busy: actions.busy,
    launching: actions.launching,
    stopping: actions.stopping,
    onStart: actions.onStart,
    onStop: actions.onStop,
    onNewSession: actions.onNewSession,
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

/** The flavor dimension of a card list: "All" plus every flavor present in it. */
export function flavorGroup(
  flavors: string[],
  value: string,
  onChange: (flavor: string) => void,
): FilterGroup {
  return {
    label: m['app.label.flavor'](),
    value,
    neutral: 'all',
    options: [
      { value: 'all', label: m['app.label.all']() },
      ...flavors.map((f) => ({
        value: f,
        label: f,
        className: 'capitalize',
      })),
    ],
    onChange,
  };
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
      ? m['app.collection.switch_to_list']()
      : m['app.collection.switch_to_grid']();
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
