import { PlusIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { useSearch } from '@/components/app-shell/search-context';
import { entryIcon } from '@/components/icons';
import { Page, Section } from '@/components/page';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
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
import { CreateEntryModal } from '@/features/entries/create-modal';

const InstanceIcon = entryIcon('instance');
const ServerIcon = entryIcon('server');

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

  const [newKind, setNewKind] = useState<'server' | 'instance'>('instance');
  const [creating, setCreating] = useState(false);
  const openNew = (kind: 'server' | 'instance') => {
    setNewKind(kind);
    setCreating(true);
  };

  return (
    <Page
      title="Library"
      subtitle="Instances you play and servers you host"
      search
      searchPlaceholder="Search library"
      actions={
        <>
          <ViewToggle view={view} onView={onViewChange} />
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button size="sm" data-icon="inline-start">
                  <PlusIcon weight="bold" />
                  New
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-44">
              <DropdownMenuItem onClick={() => openNew('instance')}>
                <InstanceIcon />
                New instance
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => openNew('server')}>
                <ServerIcon />
                New server
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
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

      <CreateEntryModal
        kind={newKind}
        open={creating}
        onOpenChange={setCreating}
      />
    </Page>
  );
}
