import { PlusIcon } from '@phosphor-icons/react';

import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  instanceCards,
  instanceFlavors,
  type View,
  ViewToggle,
} from '@/features/entries/collection';

export function InstancesPage({
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
  const cards = filterCards(instanceCards, query, flavor);

  return (
    <Page
      title="Instances"
      subtitle="Worlds you play"
      search
      searchPlaceholder="Search instances"
      actions={
        <>
          <FilterMenu
            groups={[
              {
                label: 'Flavor',
                flavors: instanceFlavors,
                value: flavor,
                onChange: onFlavorChange,
              },
            ]}
          />
          <ViewToggle view={view} onView={onViewChange} />
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
