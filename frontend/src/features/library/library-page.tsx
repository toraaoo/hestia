import { PlusIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { useSearch } from '@/components/app-shell/search-context';
import { Page, Section } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  EntryCollection,
  FilterMenu,
  filterCards,
  instanceCards,
  instanceFlavors,
  serverCards,
  serverFlavors,
  type View,
  ViewToggle,
} from '@/features/entries/collection';

export function LibraryPage({
  view,
  serverFlavor,
  instanceFlavor,
  onViewChange,
  onServerFlavorChange,
  onInstanceFlavorChange,
}: {
  view: View;
  serverFlavor: string;
  instanceFlavor: string;
  onViewChange: (view: View) => void;
  onServerFlavorChange: (flavor: string) => void;
  onInstanceFlavorChange: (flavor: string) => void;
}) {
  const { query } = useSearch();

  const srv = filterCards(serverCards, query, serverFlavor);
  const inst = filterCards(instanceCards, query, instanceFlavor);

  return (
    <Page
      title="Library"
      subtitle="Instances you play and servers you host"
      search
      searchPlaceholder="Search library"
      actions={
        <>
          <ViewToggle view={view} onView={onViewChange} />
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
              <FilterMenu
                groups={[
                  {
                    label: 'Flavor',
                    flavors: serverFlavors,
                    value: serverFlavor,
                    onChange: onServerFlavorChange,
                  },
                ]}
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
            <div className="flex items-center gap-3">
              <FilterMenu
                groups={[
                  {
                    label: 'Flavor',
                    flavors: instanceFlavors,
                    value: instanceFlavor,
                    onChange: onInstanceFlavorChange,
                  },
                ]}
              />
              <Link
                to="/instances"
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Manage all
              </Link>
            </div>
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
