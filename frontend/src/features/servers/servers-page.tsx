import { PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FlavorFilter,
  filterCards,
  serverCards,
  serverFlavors,
  type View,
  ViewToggle,
} from '@/features/entries/collection';

export function ServersPage() {
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
