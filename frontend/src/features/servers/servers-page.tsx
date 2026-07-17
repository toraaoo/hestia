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
import { m } from '@/paraglide/messages.js';

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
      title={m['nav.servers']()}
      subtitle={m['servers.subtitle']()}
      search
      searchPlaceholder={m['search.servers']()}
      actions={
        <>
          <FilterMenu
            groups={[
              {
                label: m['label.flavor'](),
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
            {m['servers.new']()}
          </Button>
        </>
      }
    >
      <EntryCollection
        cards={cards}
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
