import { PlusIcon } from '@phosphor-icons/react';
import { useMemo, useState } from 'react';
import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  flavorsOf,
  serverToCard,
  type View,
  ViewToggle,
} from '@/features/entries/collection';
import { CreateEntryModal } from '@/features/entries/create';
import type { EntryCardData } from '@/features/entries/entry-card';
import { EntryGridSkeleton } from '@/features/entries/skeleton';
import { m } from '@/paraglide/messages.js';
import {
  useServers,
  useStartServerAny,
  useStopServerAny,
} from '@/queries/server';

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
        ...serverToCard(server, {
          busy: busyId === server.id,
          onStart: () => start.mutate(server.id),
          onStop: () => stop.mutate(server.id),
        }),
        iconUrl: server.iconUrl,
      })),
    [servers.data, busyId, start, stop],
  );

  const flavors = useMemo(() => flavorsOf(cards), [cards]);
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
