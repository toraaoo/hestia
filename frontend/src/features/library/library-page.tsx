import { PlusIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { Page, Section } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FlavorFilter,
  filterCards,
  instanceCards,
  instanceFlavors,
  serverCards,
  serverFlavors,
  type View,
  ViewToggle,
} from '@/features/entries/collection';

export function LibraryPage() {
  const { query } = useSearch();
  const [view, setView] = useState<View>('grid');
  const [instFlavor, setInstFlavor] = useState('all');
  const [srvFlavor, setSrvFlavor] = useState('all');

  const srv = filterCards(serverCards, query, srvFlavor);
  const inst = filterCards(instanceCards, query, instFlavor);

  return (
    <Page
      title="Library"
      subtitle="Instances you play and servers you host"
      search
      searchPlaceholder="Search library"
      actions={
        <>
          <ViewToggle view={view} onView={setView} />
          <Button size="sm" data-icon="inline-start">
            <PlusIcon weight="bold" />
            New
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-6">
        <Section
          title="Your servers"
          count={srv.length}
          action={
            <div className="flex items-center gap-3">
              <FlavorFilter
                value={srvFlavor}
                onChange={setSrvFlavor}
                flavors={serverFlavors}
              />
              <Link
                to="/servers"
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Manage all
              </Link>
            </div>
          }
        >
          <EntryCollection
            cards={srv}
            view={view}
            empty="No servers match your search."
          />
        </Section>

        <Section
          title="Instances"
          count={inst.length}
          action={
            <FlavorFilter
              value={instFlavor}
              onChange={setInstFlavor}
              flavors={instanceFlavors}
            />
          }
        >
          <EntryCollection
            cards={inst}
            view={view}
            empty="No instances match your search."
          />
        </Section>
      </div>
    </Page>
  );
}
