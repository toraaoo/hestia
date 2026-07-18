import { PlusIcon } from '@phosphor-icons/react';
import { useMemo, useState } from 'react';

import type { InstanceInfo } from '@/api';
import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { SignInGate } from '@/components/sign-in-gate';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  type View,
  ViewToggle,
} from '@/features/entries/collection';
import { CreateEntryModal } from '@/features/entries/create-modal';
import type { EntryCardData } from '@/features/entries/entry-card';
import { EntryGridSkeleton } from '@/features/entries/skeleton';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import {
  useInstances,
  useLaunchInstanceAny,
  useStopInstanceAny,
} from '@/queries/instance';

function runningSessions(instance: InstanceInfo): number {
  return (instance.sessions ?? []).filter((s) => s.state === 'running').length;
}

function subtitle(instance: InstanceInfo): string {
  const running = runningSessions(instance);
  return running > 0
    ? m['entry.sessions_running']({ count: running })
    : m['status.stopped']();
}

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
  const launch = useLaunchInstanceAny();
  const stop = useStopInstanceAny();
  const [creating, setCreating] = useState(false);

  const busyId =
    launch.isPending || stop.isPending
      ? ((launch.variables ?? stop.variables) as string | undefined)
      : undefined;

  const cards: EntryCardData[] = useMemo(
    () =>
      (instances.data ?? []).map((instance) => ({
        id: instance.id,
        name: instance.name,
        kind: 'instance' as const,
        flavor: instance.flavor,
        version: instance.gameVersion,
        running: runningSessions(instance) > 0,
        ready: true,
        subtitle: subtitle(instance),
        busy: busyId === instance.id,
        onStart: () => launch.mutate(instance.id),
        onStop: () => stop.mutate(instance.id),
      })),
    [instances.data, busyId, launch, stop],
  );

  const flavors = useMemo(
    () => [...new Set(cards.map((c) => c.flavor))],
    [cards],
  );
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
