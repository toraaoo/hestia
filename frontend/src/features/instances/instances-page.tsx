import { PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { SignInGate } from '@/components/sign-in-gate';
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
import { EntryGridSkeleton } from '@/features/entries/skeleton';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';

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
  const { signedIn, ready } = useAccounts();

  return (
    <Page
      title={m['nav.instances']()}
      subtitle={m['instances.subtitle']()}
      loading={!ready}
      skeleton={<EntryGridSkeleton />}
      search={signedIn}
      searchPlaceholder={m['search.instances']()}
      actions={
        signedIn ? (
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
        ) : undefined
      }
    >
      {signedIn ? (
        <>
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
        </>
      ) : (
        <SignInGate
          title={m['instances.locked_title']()}
          hint={m['instances.sign_in_hint']()}
        />
      )}
    </Page>
  );
}
