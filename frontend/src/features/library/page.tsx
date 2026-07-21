import { PlusIcon, SignInIcon } from '@phosphor-icons/react';
import { useMutation } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
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
  flavorsOf,
  instanceToCard,
  serverToCard,
  type View,
  ViewToggle,
} from '@/features/entries/components/collection';
import type { EntryCardModel } from '@/features/entries/components/entry-card';
import { EntryGridSkeleton } from '@/features/entries/components/skeleton';
import { CreateEntryModal } from '@/features/entries/create';
import { useLaunchModal } from '@/features/instances/launch-modal';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { instanceMutations, useInstances } from '@/queries/instance';
import { serverMutations, useServers } from '@/queries/server';

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
  const { signedIn, ready } = useAccounts();

  const servers = useServers();
  const startServer = useMutation(serverMutations.startAny());
  const stopServer = useMutation(serverMutations.stopAny());

  const instances = useInstances();
  const { launch: launchInstance, isLaunching } = useLaunchModal();
  const stopInstance = useMutation(instanceMutations.stopAny());

  const [newKind, setNewKind] = useState<'server' | 'instance'>('instance');
  const [creating, setCreating] = useState(false);
  const openNew = (kind: 'server' | 'instance') => {
    setNewKind(kind);
    setCreating(true);
  };

  const serverBusy =
    startServer.isPending || stopServer.isPending
      ? ((startServer.variables ?? stopServer.variables) as string | undefined)
      : undefined;
  const serverCards: EntryCardModel[] = useMemo(
    () =>
      (servers.data ?? []).map((server) => ({
        ...serverToCard(server, {
          busy: serverBusy === server.id,
          onStart: () => startServer.mutate(server.id),
          onStop: () => stopServer.mutate(server.id),
        }),
        iconUrl: server.iconUrl,
      })),
    [servers.data, serverBusy, startServer, stopServer],
  );

  const instanceCards: EntryCardModel[] = useMemo(
    () =>
      (instances.data ?? []).map((instance) => ({
        ...instanceToCard(
          instance,
          {
            busy:
              isLaunching(instance.id) ||
              (stopInstance.isPending &&
                stopInstance.variables === instance.id),
            onStart: () => launchInstance(instance),
            onStop: () => stopInstance.mutate(instance.id),
          },
          instance.lastPlayedUnix,
        ),
        iconUrl: instance.iconUrl,
      })),
    [instances.data, isLaunching, launchInstance, stopInstance],
  );

  const serverFlavors = useMemo(() => flavorsOf(serverCards), [serverCards]);
  const instanceFlavors = useMemo(
    () => flavorsOf(instanceCards),
    [instanceCards],
  );

  const srv = filterCards(serverCards, query, serverFlavor);
  const inst = filterCards(instanceCards, query, instanceFlavor);

  const loading =
    !ready || servers.isPending || (signedIn && instances.isPending);

  return (
    <Page
      title={m['nav.library']()}
      subtitle={m['library.subtitle']()}
      loading={loading}
      skeleton={
        <div className="flex flex-col gap-6">
          <EntryGridSkeleton header count={4} />
          <EntryGridSkeleton header count={4} />
        </div>
      }
      search
      searchPlaceholder={m['search.library']()}
      actions={
        <>
          <ViewToggle view={view} onView={onViewChange} />
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button size="sm" data-icon="inline-start">
                  <PlusIcon weight="bold" />
                  {m['library.new']()}
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-44">
              <DropdownMenuItem
                disabled={!signedIn}
                onClick={() => openNew('instance')}
              >
                <InstanceIcon />
                {m['instances.new']()}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => openNew('server')}>
                <ServerIcon />
                {m['servers.new']()}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </>
      }
    >
      <div className="flex flex-col gap-6">
        <Section
          title={m['nav.instances']()}
          count={signedIn ? inst.length : undefined}
          action={
            signedIn ? (
              <div className="flex items-center gap-3">
                <FilterMenu
                  groups={[
                    {
                      label: m['label.flavor'](),
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
                  {m['library.manage_all']()}
                </Link>
              </div>
            ) : undefined
          }
        >
          {signedIn ? (
            <EntryCollection
              cards={inst}
              view={view}
              empty={m['instances.none_match']()}
            />
          ) : (
            <InstancesSignInPrompt />
          )}
        </Section>

        <Section
          title={m['library.your_servers']()}
          count={srv.length}
          action={
            <div className="flex items-center gap-3">
              <FilterMenu
                groups={[
                  {
                    label: m['label.flavor'](),
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
                {m['library.manage_all']()}
              </Link>
            </div>
          }
        >
          <EntryCollection
            cards={srv}
            view={view}
            empty={m['servers.none_match']()}
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

/** Instances need a signed-in account, so their section blocks until sign-in. */
function InstancesSignInPrompt() {
  const { login } = useAccounts();
  return (
    <div className="flex flex-col items-center gap-4 border border-dashed border-border px-4 py-10 text-center">
      <div className="space-y-1">
        <p className="text-sm font-medium">{m['instances.locked_title']()}</p>
        <p className="text-xs text-muted-foreground">
          {m['instances.sign_in_hint']()}
        </p>
      </div>
      <Button
        size="sm"
        data-icon="inline-start"
        disabled={login.isPending}
        onClick={() => login.mutate()}
      >
        <SignInIcon weight="bold" />
        {login.isPending ? m['account.signing_in']() : m['account.sign_in']()}
      </Button>
    </div>
  );
}
