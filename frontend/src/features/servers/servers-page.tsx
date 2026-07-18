import { PlusIcon } from '@phosphor-icons/react';
import { useMemo, useState } from 'react';
import type { ServerInfo } from '@/api';
import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  type View,
  ViewToggle,
} from '@/features/entries/collection';
import { CreateEntryModal } from '@/features/entries/create-modal';
import type { EntryCardData } from '@/features/entries/entry-card';
import { EntryGridSkeleton } from '@/features/entries/skeleton';
import { m } from '@/paraglide/messages.js';
import {
  useServers,
  useStartServerAny,
  useStopServerAny,
} from '@/queries/server';

function isRunning(server: ServerInfo): boolean {
  return server.process?.state === 'running';
}

function subtitle(server: ServerInfo): string {
  if (!server.ready) return m['status.preparing_ellipsis']();
  const address = server.game_port ? `:${server.game_port}` : '';
  const state = isRunning(server)
    ? m['status.online']()
    : m['status.stopped']();
  return address ? `${address} · ${state}` : state;
}

export function ServersPage({
  view,
  flavor,
  onViewChange,
  onFlavorChange,
}: {
  view: View;
  flavor: string;
  onViewChange: (view: View) => void;
  onFlavorChange: (flavor: string) => void;
}) {
  const { query } = useSearch();
  const servers = useServers();
  const start = useStartServerAny();
  const stop = useStopServerAny();
  const [creating, setCreating] = useState(false);

  const busyId =
    start.isPending || stop.isPending
      ? ((start.variables ?? stop.variables) as string | undefined)
      : undefined;

  const cards: EntryCardData[] = useMemo(
    () =>
      (servers.data ?? []).map((server) => ({
        id: server.id,
        name: server.name,
        kind: 'server' as const,
        flavor: server.flavor,
        version: server.game_version,
        running: isRunning(server),
        ready: server.ready,
        subtitle: subtitle(server),
        busy: busyId === server.id,
        onStart: () => start.mutate(server.id),
        onStop: () => stop.mutate(server.id),
      })),
    [servers.data, busyId, start, stop],
  );

  const flavors = useMemo(
    () => [...new Set(cards.map((c) => c.flavor))],
    [cards],
  );
  const filtered = filterCards(cards, query, flavor);

  return (
    <Page
      title={m['nav.servers']()}
      subtitle={m['servers.subtitle']()}
      skeleton={<EntryGridSkeleton />}
      loading={servers.isPending}
      search
      searchPlaceholder={m['search.servers']()}
      actions={
        <>
          <FilterMenu
            groups={[
              {
                label: m['label.flavor'](),
                flavors,
                value: flavor,
                onChange: onFlavorChange,
              },
            ]}
          />
          <ViewToggle view={view} onView={onViewChange} />
          <Button
            size="sm"
            data-icon="inline-start"
            onClick={() => setCreating(true)}
          >
            <PlusIcon weight="bold" />
            {m['servers.new']()}
          </Button>
        </>
      }
    >
      <EntryCollection
        cards={filtered}
        view={view}
        empty={m['servers.none_match']()}
      />
      <CreateEntryModal
        kind="server"
        open={creating}
        onOpenChange={setCreating}
      />
    </Page>
  );
}
