import { PlusIcon } from '@phosphor-icons/react';
import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';

import {
  EntryCollection,
  FlavorFilter,
  filterCards,
  instanceCards,
  instanceFlavors,
  type View,
  ViewToggle,
} from '@/components/launcher/library';
import { Page } from '@/components/launcher/page';
import { useSearch } from '@/components/launcher/search-context';
import { Button } from '@/components/ui/button';

export const Route = createFileRoute('/_app/instances/')({
  component: InstancesPage,
});

function InstancesPage() {
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
