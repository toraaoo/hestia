import { PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

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
import { CreateEntryModal } from '@/features/entries/create-modal';
import { m } from '@/paraglide/messages.js';

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
  const [creating, setCreating] = useState(false);

  return (
    <Page
      title={m['nav.instances']()}
      subtitle={m['instances.subtitle']()}
      search
      searchPlaceholder={m['search.instances']()}
      actions={
        <>
          <FilterMenu
            groups={[
              {
                label: m['label.flavor'](),
                flavors: instanceFlavors,
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
            {m['instances.new']()}
          </Button>
        </>
      }
    >
      <EntryCollection
        cards={cards}
        view={view}
        empty={m['instances.none_match']()}
      />
      <CreateEntryModal
        kind="instance"
        open={creating}
        onOpenChange={setCreating}
      />
    </Page>
  );
}
