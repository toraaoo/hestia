import { PlusIcon } from '@phosphor-icons/react';
import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';

import {
  EntryCollection,
  FlavorFilter,
  filterCards,
  serverCards,
  serverFlavors,
  type View,
  ViewToggle,
} from '@/components/launcher/library';
import { Page } from '@/components/launcher/page';
import { useSearch } from '@/components/launcher/search-context';
import { Button } from '@/components/ui/button';

export const Route = createFileRoute('/_app/servers/')({
  component: ServersPage,
});

function ServersPage() {
  const { query } = useSearch();
  const [view, setView] = useState<View>('grid');
  const [flavor, setFlavor] = useState('all');
  const cards = filterCards(serverCards, query, flavor);

  return (
    <Page
      title="Servers"
      subtitle="Worlds you host"
      search
      searchPlaceholder="Search servers"
      actions={
        <>
          <FlavorFilter
            value={flavor}
            onChange={setFlavor}
            flavors={serverFlavors}
          />
          <ViewToggle view={view} onView={setView} />
          <Button size="sm" data-icon="inline-start">
            <PlusIcon weight="bold" />
            New server
          </Button>
        </>
      }
    >
      <EntryCollection
        cards={cards}
        view={view}
        empty="No servers match your search."
      />
    </Page>
  );
}
