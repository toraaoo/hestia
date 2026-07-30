import { PlusIcon } from '@phosphor-icons/react';
import { useMutation } from '@tanstack/react-query';
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
} from '@/features/entries/components/collection';
import type { EntryCardModel } from '@/features/entries/components/entry-card';
import { EntryGridSkeleton } from '@/features/entries/components/skeleton';
import { CreateEntryModal } from '@/features/entries/create';
import { m } from '@/paraglide/messages.js';
import { serverMutations, useServers } from '@/queries/server';

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
  const start = useMutation(serverMutations.startAny());
  const stop = useMutation(serverMutations.stopAny());
  const [creating, setCreating] = useState(false);

  const busyId =
    start.isPending || stop.isPending
      ? ((start.variables ?? stop.variables) as string | undefined)
      : undefined;

  const cards: EntryCardModel[] = useMemo(
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
      title={m['app.nav.servers']()}
      subtitle={m['server.subtitle']()}
      skeleton={<EntryGridSkeleton />}
      loading={servers.isPending}
      search
      searchPlaceholder={m['app.search.servers']()}
      actions={
        <>
          <FilterMenu
            groups={[
              {
                label: m['app.label.flavor'](),
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
            {m['server.new']()}
          </Button>
        </>
      }
    >
      <EntryCollection
        cards={filtered}
        view={view}
        empty={m['server.none_match']()}
      />
      <CreateEntryModal
        kind="server"
        open={creating}
        onOpenChange={setCreating}
      />
    </Page>
  );
}
