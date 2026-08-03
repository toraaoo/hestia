import {
  ArrowsClockwiseIcon,
  HardDrivesIcon,
  PlusIcon,
} from '@phosphor-icons/react';
import {
  useIsFetching,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import { Reorder } from 'motion/react';
import { useEffect, useMemo, useState } from 'react';

import type { InstanceInfo, ServerEntry } from '@/api';
import { Empty } from '@/components/empty';
import { Bone } from '@/components/skeleton';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { ServerRow } from '@/features/instances/components';
import {
  ServerEntryDialog,
  useLaunchDialog,
} from '@/features/instances/dialogs';
import { supportsQuickPlay } from '@/lib/quick-play';
import { runningSessions } from '@/lib/sessions';
import { toastWarnings } from '@/lib/warnings';
import { m } from '@/paraglide/messages.js';
import { instanceMutations, instanceQueries } from '@/queries/instance';
import { keys } from '@/queries/keys';

/** The identity a row is dragged and referenced by; the daemon takes either half. */
const keyOf = (server: ServerEntry) => `${server.name}:${server.address}`;

/**
 * The instance's multiplayer list — the servers the in-game list shows, read
 * from its own `servers.dat`. Each row carries the server's icon and what it
 * answers right now, can be joined straight from here (Quick Play), and can be
 * dragged into the order the game will show.
 *
 * The file belongs to the running game, which rewrites it wholesale on exit,
 * so an edit made while a session is open comes back with a warning saying so.
 */
export function InstanceServersTab({ instance }: { instance: InstanceInfo }) {
  const client = useQueryClient();
  const servers = useQuery(instanceQueries.servers(instance.id));
  const { launch, isLaunching } = useLaunchDialog();
  const remove = useMutation(instanceMutations.serverRemove(instance.id));
  const move = useMutation(instanceMutations.serverMove(instance.id));
  const [editing, setEditing] = useState<ServerEntry | null | undefined>(
    undefined,
  );
  const [removing, setRemoving] = useState<ServerEntry | null>(null);

  const joinable = supportsQuickPlay(instance.gameVersion);
  const running = runningSessions(instance).length > 0;
  // The game's own scratch rows (direct-connect) are not the player's servers.
  const list = useMemo(
    () => (servers.data ?? []).filter((server) => !server.hidden),
    [servers.data],
  );
  const order = useServerOrder(list);

  const commit = (server: ServerEntry, position: number) =>
    move.mutate(
      { server: server.name || server.address, position },
      {
        onSuccess: (written) => toastWarnings(written.warnings),
        // The preview outlives a failed write otherwise: the list comes back
        // in its old order, which the reconciling effect never agrees with.
        onError: () => order.reset(),
      },
    );

  // A refresh re-reads the file and re-pings every address; the statuses are
  // keyed per address rather than per instance, so they sweep on their own key.
  const pinging = useIsFetching({ queryKey: keys.instances.serverStatuses() });
  const refresh = () => {
    client.invalidateQueries({ queryKey: keys.instances.servers(instance.id) });
    client.invalidateQueries({ queryKey: keys.instances.serverStatuses() });
  };

  return (
    <div className="flex flex-1 flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {m['instance.servers.summary']()}
        </p>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={m['app.action.refresh']()}
            title={m['app.action.refresh']()}
            disabled={servers.isPending || pinging > 0}
            onClick={refresh}
          >
            <ArrowsClockwiseIcon
              className={pinging > 0 ? 'animate-spin' : undefined}
            />
          </Button>
          <Button
            size="sm"
            data-icon="inline-start"
            onClick={() => setEditing(null)}
          >
            <PlusIcon weight="bold" />
            {m['instance.servers.add.action']()}
          </Button>
        </div>
      </div>

      {servers.isPending ? (
        <div className="space-y-2">
          <Bone className="h-22" />
          <Bone className="h-22" />
        </div>
      ) : list.length === 0 ? (
        <Empty className="flex-1" icon={HardDrivesIcon}>
          {m['instance.servers.empty']()}
        </Empty>
      ) : (
        <Reorder.Group
          as="div"
          axis="y"
          values={order.keys}
          onReorder={order.preview}
          className="divide-y divide-border border border-border"
        >
          {order.rows.map((server, index) => (
            <ServerRow
              key={keyOf(server)}
              server={server}
              joinable={joinable}
              playing={isLaunching(instance.id, server.address)}
              first={index === 0}
              last={index === order.rows.length - 1}
              onPlay={() =>
                launch(instance, {
                  quickPlay: { kind: 'server', target: server.address },
                  newSession: running,
                })
              }
              onEdit={() => setEditing(server)}
              onRemove={() => setRemoving(server)}
              onMoveUp={() => commit(server, index - 1)}
              onMoveDown={() => commit(server, index + 1)}
              onDragStart={() => order.onDragStart(keyOf(server))}
              onDragEnd={() => {
                const moved = order.settle();
                if (moved) commit(moved.server, moved.position);
              }}
            />
          ))}
        </Reorder.Group>
      )}

      <ServerEntryDialog
        instance={instance.id}
        server={editing}
        onOpenChange={(open) => {
          if (!open) setEditing(undefined);
        }}
      />
      <ConfirmDialog
        open={removing != null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null);
        }}
        title={m['instance.servers.remove.title']({
          name: removing?.name ?? '',
        })}
        description={m['instance.servers.remove.description']()}
        confirmLabel={m['app.action.remove']()}
        destructive
        onConfirm={() => {
          const target = removing;
          if (!target) return;
          remove.mutate(target.name || target.address, {
            onSuccess: (written) => toastWarnings(written.warnings),
          });
          setRemoving(null);
        }}
      />
    </div>
  );
}

/**
 * The list's drag order.
 *
 * `Reorder` reports a new order on every crossing, so the drag runs against a
 * local preview keyed by row and only the release is written — one call per
 * gesture rather than one per row swap. The preview clears once the refetched
 * list agrees with it, which is what keeps the rows from snapping back while
 * the write is still in flight.
 */
function useServerOrder(servers: ServerEntry[]) {
  const [order, setOrder] = useState<string[] | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);

  const rows = useMemo(() => {
    if (!order) return servers;
    const byKey = new Map(servers.map((server) => [keyOf(server), server]));
    const preview = order.flatMap((key) => byKey.get(key) ?? []);
    return preview.length === servers.length ? preview : servers;
  }, [order, servers]);

  useEffect(() => {
    if (order && order.join() === servers.map(keyOf).join()) setOrder(null);
  }, [order, servers]);

  return {
    rows,
    keys: useMemo(() => rows.map(keyOf), [rows]),
    preview: setOrder,
    reset: () => setOrder(null),
    onDragStart: setDragging,
    /**
     * Where the dragged row landed, or `null` when the gesture left the order
     * as it was — writing that back would be a rewrite of the game's file for
     * nothing.
     */
    settle: () => {
      const key = dragging;
      setDragging(null);
      if (!key) return null;
      const position = rows.findIndex((server) => keyOf(server) === key);
      const server = rows[position];
      if (!server || servers.findIndex((s) => keyOf(s) === key) === position) {
        return null;
      }
      return { server, position };
    },
  };
}
