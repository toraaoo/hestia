import { CaretUpDownIcon, PlayIcon, PowerIcon } from '@phosphor-icons/react';
import { useMutation } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import type { InstanceInfo } from '@/api';
import { entryIcon } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { useLaunchModal } from '@/features/instances/launch-modal';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { instanceMutations, useInstances } from '@/queries/instance';

function isRunning(instance: InstanceInfo): boolean {
  return (instance.sessions ?? []).some((s) => s.state === 'running');
}

/**
 * The always-present quick-play strip along the bottom of the library. The
 * instance is chosen from a dropdown; the button launches it (or stops it when
 * a session is already running).
 */
export function PlayBar() {
  const { signedIn } = useAccounts();
  const instances = useInstances();
  const { launch, isLaunching } = useLaunchModal();
  const stop = useMutation(instanceMutations.stopAny());

  const list = instances.data ?? [];
  const [selId, setSelId] = useState<string | null>(null);
  const sel = list.find((i) => i.id === selId) ?? list[0];

  const Icon = entryIcon('instance');
  const running = sel ? isRunning(sel) : false;
  const busy =
    (sel ? isLaunching(sel.id) : false) ||
    (stop.isPending && stop.variables === sel?.id);

  return (
    <div className="flex h-[76px] items-center gap-3 border-t border-border bg-sidebar px-4">
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <button
              type="button"
              disabled={list.length === 0}
              className="-ml-2 flex h-14 w-72 items-center gap-3 px-2 text-left transition-colors outline-none hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset aria-expanded:bg-muted disabled:opacity-50"
            >
              <span className="grid size-11 shrink-0 place-items-center overflow-hidden bg-muted text-muted-foreground ring-1 ring-border">
                {sel?.iconUrl ? (
                  <img
                    src={sel.iconUrl}
                    alt=""
                    className="size-full object-cover"
                  />
                ) : (
                  <Icon className="size-6" />
                )}
              </span>
              <span className="min-w-0 flex-1 leading-tight">
                <span className="block text-[11px] tracking-wide text-muted-foreground uppercase">
                  {m['playbar.quick_play']()}
                </span>
                <span className="block truncate text-base font-medium">
                  {sel?.name ?? '—'}
                </span>
              </span>
              <CaretUpDownIcon className="size-4 shrink-0 text-muted-foreground" />
            </button>
          }
        />
        <DropdownMenuContent side="top" align="start" className="w-56">
          <DropdownMenuGroup>
            <DropdownMenuLabel>
              {m['playbar.all_instances']()}
            </DropdownMenuLabel>
            {list.map((i) => (
              <InstanceItem
                key={i.id}
                instance={i}
                onSelect={() => setSelId(i.id)}
              />
            ))}
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>

      <div className="ml-auto hidden items-center gap-1.5 sm:flex">
        {sel && (
          <>
            <Badge variant="secondary" className="uppercase">
              {sel.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {sel.gameVersion}
            </Badge>
          </>
        )}
      </div>

      {sel && (
        <Button
          variant="ghost"
          size="sm"
          nativeButton={false}
          render={<Link to="/instances/$id" params={{ id: sel.id }} />}
        >
          {m['action.manage']()}
        </Button>
      )}

      {running ? (
        <ConfirmDialog
          trigger={
            <Button
              variant="outline"
              size="sm"
              data-icon="inline-start"
              disabled={busy}
            >
              <PowerIcon weight="bold" />
              {m['action.stop']()}
            </Button>
          }
          title={m['entry.stop_title']({ name: sel?.name ?? '' })}
          description={m['entry.stop_instance_description']()}
          confirmLabel={m['action.stop']()}
          onConfirm={() => sel && stop.mutate(sel.id)}
        />
      ) : (
        <Button
          data-icon="inline-start"
          disabled={!signedIn || !sel || busy}
          title={signedIn ? undefined : m['playbar.sign_in_required']()}
          onClick={() => sel && launch(sel)}
          className="bg-ember text-ember-foreground hover:bg-ember/90"
        >
          {busy ? <Spinner /> : <PlayIcon weight="fill" />}
          {m['action.play']()}
        </Button>
      )}
    </div>
  );
}

function InstanceItem({
  instance,
  onSelect,
}: {
  instance: InstanceInfo;
  onSelect: () => void;
}) {
  return (
    <DropdownMenuItem onClick={onSelect}>
      <span className="min-w-0 flex-1 truncate">{instance.name}</span>
      {isRunning(instance) ? (
        <StatusDot tone="on" />
      ) : (
        <span className="font-mono text-[10px] text-muted-foreground">
          {instance.gameVersion}
        </span>
      )}
    </DropdownMenuItem>
  );
}
