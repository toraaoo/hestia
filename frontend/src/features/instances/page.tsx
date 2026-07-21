import { PlusIcon } from '@phosphor-icons/react';
import { useMemo, useState } from 'react';
import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { SignInGate } from '@/components/sign-in-gate';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  flavorsOf,
  instanceToCard,
  type View,
  ViewToggle,
} from '@/features/entries/collection';
import { CreateEntryModal } from '@/features/entries/create';
import type { EntryCardData } from '@/features/entries/entry-card';
import { EntryGridSkeleton } from '@/features/entries/skeleton';
import { useLaunchModal } from '@/features/instances/launch-modal';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { useInstances, useStopInstanceAny } from '@/queries/instance';

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
  const { signedIn, ready } = useAccounts();
  const instances = useInstances();
  const { launch, isLaunching } = useLaunchModal();
  const stop = useStopInstanceAny();
  const [creating, setCreating] = useState(false);

  const cards: EntryCardData[] = useMemo(
    () =>
      (instances.data ?? []).map((instance) => ({
        ...instanceToCard(
          instance,
          {
            busy:
              isLaunching(instance.id) ||
              (stop.isPending && stop.variables === instance.id),
            onStart: () => launch(instance),
            onStop: () => stop.mutate(instance.id),
          },
          instance.lastPlayedUnix,
        ),
        iconUrl: instance.iconUrl,
      })),
    [instances.data, isLaunching, launch, stop],
  );

  const flavors = useMemo(() => flavorsOf(cards), [cards]);
  const filtered = filterCards(cards, query, flavor);

  return (
    <Page
      title={m['nav.instances']()}
      subtitle={m['instances.subtitle']()}
      loading={!ready || (signedIn && instances.isPending)}
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
              {m['instances.new']()}
            </Button>
          </>
        ) : undefined
      }
    >
      {signedIn ? (
        <>
          <EntryCollection
            cards={filtered}
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
