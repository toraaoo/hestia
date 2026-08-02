import {
  HardDrivesIcon,
  PlayIcon,
  PlusIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import type { InstanceInfo, ServerEntry } from '@/api';
import { Empty } from '@/components/empty';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { Switch } from '@/components/ui/switch';
import { supportsQuickPlay } from '@/lib/quick-play';
import { runningSessions } from '@/lib/sessions';
import { toastWarnings } from '@/lib/warnings';
import { m } from '@/paraglide/messages.js';
import { instanceMutations, instanceQueries } from '@/queries/instance';
import { useJobMutation } from '@/queries/jobs';

/**
 * The instance's multiplayer list — the servers the in-game list shows, read
 * from its own `servers.dat`. Each row carries what the server answers right
 * now and can be joined straight from here (Quick Play).
 *
 * The file belongs to the running game, which rewrites it wholesale on exit,
 * so an edit made while a session is open comes back with a warning saying so.
 */
export function InstanceServersTab({ instance }: { instance: InstanceInfo }) {
  const servers = useQuery(instanceQueries.servers(instance.id));
  const launch = useJobMutation(instanceMutations.launchQuick());
  const remove = useMutation(instanceMutations.serverRemove(instance.id));
  const [editing, setEditing] = useState<ServerEntry | null | undefined>(
    undefined,
  );
  const [removing, setRemoving] = useState<ServerEntry | null>(null);

  const joinable = supportsQuickPlay(instance.gameVersion);
  const running = runningSessions(instance).length > 0;
  // The game's own scratch rows (direct-connect) are not the player's servers.
  const list = (servers.data ?? []).filter((server) => !server.hidden);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {m['instance.servers.summary']()}
        </p>
        <Button
          size="sm"
          data-icon="inline-start"
          onClick={() => setEditing(null)}
        >
          <PlusIcon weight="bold" />
          {m['instance.servers.add.action']()}
        </Button>
      </div>

      {servers.isPending ? (
        <div className="space-y-2">
          <Bone className="h-10" />
          <Bone className="h-10" />
        </div>
      ) : list.length === 0 ? (
        <Empty icon={HardDrivesIcon}>{m['instance.servers.empty']()}</Empty>
      ) : (
        <div className="divide-y divide-border border border-border">
          {list.map((server) => (
            <ServerRow
              key={`${server.name}:${server.address}`}
              server={server}
              joinable={joinable}
              playing={
                launch.isPending &&
                launch.variables?.quickPlay.target === server.address
              }
              onPlay={() =>
                launch.mutate(
                  {
                    id: instance.id,
                    quickPlay: { kind: 'server', target: server.address },
                    newSession: running,
                  },
                  { onSuccess: (done) => toastWarnings(done.warnings) },
                )
              }
              onEdit={() => setEditing(server)}
              onRemove={() => setRemoving(server)}
            />
          ))}
        </div>
      )}

      <ServerDialog
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

function ServerRow({
  server,
  joinable,
  playing,
  onPlay,
  onEdit,
  onRemove,
}: {
  server: ServerEntry;
  joinable: boolean;
  playing: boolean;
  onPlay: () => void;
  onEdit: () => void;
  onRemove: () => void;
}) {
  const status = useQuery(instanceQueries.serverStatus(server.address));
  const online = status.isSuccess;

  return (
    <div className="flex items-center gap-3 px-3 py-2.5">
      <StatusDot tone={online ? 'on' : 'off'} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm">{server.name}</span>
          {server.acceptTextures && (
            <Badge variant="outline" className="shrink-0">
              {m['instance.servers.textures']()}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {server.address}
          {online && ` · ${status.data.motd}`}
        </div>
      </div>
      <div className="shrink-0 text-right text-[11px] text-muted-foreground">
        {status.isPending ? (
          <Spinner />
        ) : online ? (
          <>
            <div>
              {m['instance.servers.players']({
                online: status.data.playersOnline,
                max: status.data.playersMax,
              })}
            </div>
            <div className="font-mono">{status.data.version}</div>
          </>
        ) : (
          <div>{m['instance.servers.offline']()}</div>
        )}
      </div>
      <Button
        size="sm"
        variant="ghost"
        data-icon="inline-start"
        className="shrink-0"
        disabled={playing || !joinable}
        title={joinable ? undefined : m['instance.quick_play.unsupported']()}
        onClick={onPlay}
      >
        {playing ? <Spinner /> : <PlayIcon weight="fill" />}
        {m['app.action.play']()}
      </Button>
      <Button size="sm" variant="ghost" className="shrink-0" onClick={onEdit}>
        {m['app.action.edit']()}
      </Button>
      <Button
        size="sm"
        variant="ghost"
        className="shrink-0"
        aria-label={m['app.action.remove']()}
        onClick={onRemove}
      >
        <TrashIcon />
      </Button>
    </div>
  );
}

/**
 * Add or edit one entry. `server` is `null` to add, an entry to edit, and
 * `undefined` when the dialog is closed.
 */
function ServerDialog({
  instance,
  server,
  onOpenChange,
}: {
  instance: string;
  server: ServerEntry | null | undefined;
  onOpenChange: (open: boolean) => void;
}) {
  const edit = useMutation(instanceMutations.serverEdit(instance));
  const [name, setName] = useState('');
  const [address, setAddress] = useState('');
  const [acceptTextures, setAcceptTextures] = useState(false);
  const open = server !== undefined;

  // Re-seed the fields from whichever entry the dialog was opened on.
  const [seeded, setSeeded] = useState<ServerEntry | null | undefined>(
    undefined,
  );
  if (open && seeded !== server) {
    setSeeded(server);
    setName(server?.name ?? '');
    setAddress(server?.address ?? '');
    setAcceptTextures(server?.acceptTextures ?? false);
  }

  const submit = () => {
    edit.mutate(
      {
        server: server ? server.name || server.address : undefined,
        name,
        address,
        acceptTextures,
      },
      {
        onSuccess: (written) => {
          toastWarnings(written.warnings);
          onOpenChange(false);
        },
      },
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {server
              ? m['instance.servers.edit.title']({ name: server.name })
              : m['instance.servers.add.title']()}
          </DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="server-name">{m['app.label.name']()}</Label>
            <Input
              id="server-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="server-address">{m['app.label.address']()}</Label>
            <Input
              id="server-address"
              value={address}
              placeholder="mc.example.net"
              onChange={(e) => setAddress(e.target.value)}
            />
          </div>
          <div className="flex items-center justify-between gap-3">
            <Label htmlFor="server-textures">
              {m['instance.servers.textures']()}
            </Label>
            <Switch
              id="server-textures"
              checked={acceptTextures}
              onCheckedChange={setAcceptTextures}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {m['app.action.cancel']()}
          </Button>
          <Button
            disabled={!name.trim() || !address.trim() || edit.isPending}
            onClick={submit}
          >
            {edit.isPending ? <Spinner /> : m['app.action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
