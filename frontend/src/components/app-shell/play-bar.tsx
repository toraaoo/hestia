import { CaretUpDownIcon, PlayIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import { entryIcon } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { StatusDot } from '@/components/ui/status-dot';
import { featured, instances, pinnedInstances } from '@/features/entries/mock';

/**
 * The always-present quick-play strip along the bottom of the library. The
 * instance is chosen from a dropdown (pinned first); the button launches it.
 */
export function PlayBar() {
  const [selId, setSelId] = useState(featured.id);
  const sel = instances.find((i) => i.id === selId) ?? featured;
  const Icon = entryIcon('instance');
  const others = instances.filter(
    (i) => !pinnedInstances.some((p) => p.id === i.id),
  );

  return (
    <div className="flex items-center gap-3 border-t border-border bg-sidebar px-4 py-2.5">
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <button
              type="button"
              className="flex items-center gap-3 pr-2 text-left outline-none focus-visible:ring-1 focus-visible:ring-ring aria-expanded:bg-muted/40"
            >
              <span className="grid size-9 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
                <Icon className="size-5" />
              </span>
              <span className="min-w-0">
                <span className="block text-[11px] tracking-wide text-muted-foreground uppercase">
                  Quick play
                </span>
                <span className="block truncate text-sm font-medium">
                  {sel.name}
                </span>
              </span>
              <CaretUpDownIcon className="size-4 shrink-0 text-muted-foreground" />
            </button>
          }
        />
        <DropdownMenuContent side="top" align="start" className="w-56">
          <DropdownMenuGroup>
            <DropdownMenuLabel>Pinned</DropdownMenuLabel>
            {pinnedInstances.map((i) => (
              <InstanceItem
                key={i.id}
                instance={i}
                onSelect={() => setSelId(i.id)}
              />
            ))}
          </DropdownMenuGroup>
          <DropdownMenuSeparator />
          <DropdownMenuGroup>
            <DropdownMenuLabel>All instances</DropdownMenuLabel>
            {others.map((i) => (
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
        <Badge variant="secondary" className="uppercase">
          {sel.flavor}
        </Badge>
        <Badge variant="outline" className="font-mono">
          {sel.game_version}
        </Badge>
      </div>

      <Button
        variant="ghost"
        size="sm"
        nativeButton={false}
        render={<Link to="/instances/$id" params={{ id: sel.id }} />}
      >
        Manage
      </Button>
      <Button
        data-icon="inline-start"
        className="bg-ember text-ember-foreground hover:bg-ember/90"
      >
        <PlayIcon weight="fill" />
        {sel.sessions > 0 ? 'Resume' : 'Play'}
      </Button>
    </div>
  );
}

function InstanceItem({
  instance,
  onSelect,
}: {
  instance: (typeof instances)[number];
  onSelect: () => void;
}) {
  return (
    <DropdownMenuItem onClick={onSelect}>
      <span className="min-w-0 flex-1 truncate">{instance.name}</span>
      {instance.running ? (
        <StatusDot tone="on" />
      ) : (
        <span className="font-mono text-[10px] text-muted-foreground">
          {instance.game_version}
        </span>
      )}
    </DropdownMenuItem>
  );
}
