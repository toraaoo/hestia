import { PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  serverCards,
  serverFlavors,
  type View,
  ViewToggle,
} from '@/features/entries/collection';
import { CreateEntryModal } from '@/features/entries/create-modal';

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
  const cards = filterCards(serverCards, query, flavor);
  const [creating, setCreating] = useState(false);

  return (
    <Page
      title="Servers"
      subtitle="Worlds you host"
      search
      searchPlaceholder="Search servers"
      actions={
        <>
          <FilterMenu
            groups={[
              {
                label: 'Flavor',
                flavors: serverFlavors,
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
      <CreateEntryModal
        kind="server"
        open={creating}
        onOpenChange={setCreating}
      />
    </Page>
  );
}
