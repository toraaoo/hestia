import { PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FlavorFilter,
  filterCards,
  instanceCards,
  instanceFlavors,
  type View,
  ViewToggle,
} from '@/features/entries/collection';

export function InstancesPage() {
  const { query } = useSearch();
  const [view, setView] = useState<View>('grid');
  const [flavor, setFlavor] = useState('all');
  const cards = filterCards(instanceCards, query, flavor);

  return (
    <Page
      title="Instances"
      subtitle="Worlds you play"
      search
      searchPlaceholder="Search instances"
      actions={
        <>
          <FlavorFilter
            value={flavor}
            onChange={setFlavor}
            flavors={instanceFlavors}
          />
          <ViewToggle view={view} onView={setView} />
          <Button size="sm" data-icon="inline-start">
            <PlusIcon weight="bold" />
            New instance
          </Button>
        </>
      }
    >
      <EntryCollection
        cards={cards}
        view={view}
        empty="No instances match your search."
      />
    </Page>
  );
}
