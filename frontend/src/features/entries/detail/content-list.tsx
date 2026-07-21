import {
  ArrowsClockwiseIcon,
  DotsThreeIcon,
  ProhibitIcon,
  SwapIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';
import { useState } from 'react';

import type { InstalledContent } from '@/api';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { kindInfo } from '@/features/content/lib/kinds';
import { ChangeVersionModal } from '@/features/content/version-modal';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

import {
  type EntryTarget,
  kindLoader,
  type RowHandlers,
  rowKey,
} from './content';

export function ContentList({
  entry,
  items,
  updatable,
  handlers,
  selected,
  onToggleSelect,
}: {
  entry: EntryTarget;
  items: InstalledContent[];
  updatable: Set<string>;
  handlers: RowHandlers;
  selected: Set<string> | null;
  onToggleSelect: (key: string) => void;
}) {
  const [changing, setChanging] = useState<InstalledContent | null>(null);

  if (items.length === 0) {
    return <Empty>{m['content.none_installed']()}</Empty>;
  }
  return (
    <>
      <div className="divide-y divide-border border border-border">
        {items.map((c) => (
          <ContentRow
            key={rowKey(c)}
            item={c}
            updatable={updatable.has(c.filename)}
            handlers={handlers}
            onChangeVersion={() => setChanging(c)}
            checked={selected ? selected.has(rowKey(c)) : undefined}
            onToggle={() => onToggleSelect(rowKey(c))}
          />
        ))}
      </div>
      <ChangeVersionModal
        item={changing}
        loader={changing ? kindLoader(changing.kind, entry.flavor) : undefined}
        gameVersion={entry.gameVersion || undefined}
        onOpenChange={(open) => !open && setChanging(null)}
        onPick={(item, version) => handlers.onSetVersion(item, version)}
      />
    </>
  );
}

function ContentRow({
  item,
  updatable,
  handlers,
  onChangeVersion,
  checked,
  onToggle,
}: {
  item: InstalledContent;
  updatable: boolean;
  handlers: RowHandlers;
  onChangeVersion: () => void;
  /** Set while the batch-select mode is active; undefined otherwise. */
  checked?: boolean;
  onToggle: () => void;
}) {
  const [removing, setRemoving] = useState(false);
  const [iconBroken, setIconBroken] = useState(false);
  const selecting = checked !== undefined;
  const Icon = contentIcon(item.kind);
  // A local-file import has no project page to open and no versions to move
  // between — its only action is enable/disable and removal.
  const platform = item.source !== 'file' && !!item.projectId;
  const showImage = !!item.iconUrl && !iconBroken;
  const body = (
    <>
      {showImage ? (
        <img
          src={item.iconUrl}
          alt=""
          onError={() => setIconBroken(true)}
          className="size-7 shrink-0 object-cover ring-1 ring-border"
        />
      ) : (
        <span className="grid size-7 shrink-0 place-items-center bg-muted text-muted-foreground ring-1 ring-border">
          <Icon className="size-4" />
        </span>
      )}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm group-hover/item:underline group-hover/item:underline-offset-2">
            {item.title}
          </span>
          {!item.enabled && (
            <Badge variant="outline" className="shrink-0">
              {m['content.disabled']()}
            </Badge>
          )}
          {item.origin && (
            <Badge variant="outline" className="shrink-0 font-mono">
              {m['profiles.origin_badge']({ name: item.origin })}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {contentKindLabel[item.kind]()} · {item.source} · {item.versionNumber}
        </div>
      </div>
    </>
  );

  if (selecting) {
    const id = `select-${rowKey(item)}`;
    return (
      <label
        htmlFor={id}
        className={cn(
          'flex cursor-pointer items-center gap-3 px-3 py-2.5 transition-colors hover:bg-muted/60',
          !item.enabled && 'opacity-60',
        )}
      >
        <Checkbox id={id} checked={checked} onCheckedChange={onToggle} />
        {body}
      </label>
    );
  }
  return (
    <div className={item.enabled ? undefined : 'opacity-60'}>
      <div className="flex items-center gap-3 px-3 py-2.5">
        {platform ? (
          <Link
            to="/browse/$kind/$id"
            params={{
              kind: kindInfo[item.kind].slug,
              id: item.slug || item.projectId,
            }}
            className="group/item flex min-w-0 flex-1 items-center gap-3 outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            {body}
          </Link>
        ) : (
          <div className="flex min-w-0 flex-1 items-center gap-3">{body}</div>
        )}

        {updatable && item.enabled && (
          <Button
            size="sm"
            variant="outline"
            data-icon="inline-start"
            onClick={() => handlers.onUpdate(item)}
          >
            <ArrowsClockwiseIcon weight="bold" />
            {m['content.update']()}
          </Button>
        )}
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={m['action.more']()}
              >
                <DotsThreeIcon weight="bold" className="size-4" />
              </Button>
            }
          />
          <DropdownMenuContent align="end" className="w-48">
            {platform && (
              <>
                {updatable && (
                  <DropdownMenuItem onClick={() => handlers.onUpdate(item)}>
                    <ArrowsClockwiseIcon />
                    {m['content.update_to_latest']()}
                  </DropdownMenuItem>
                )}
                <DropdownMenuItem onClick={onChangeVersion}>
                  <SwapIcon />
                  {m['content.change_version']()}
                </DropdownMenuItem>
              </>
            )}
            <DropdownMenuItem
              onClick={() => handlers.onEnable(item, !item.enabled)}
            >
              <ProhibitIcon />
              {item.enabled ? m['content.disable']() : m['content.enable']()}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              variant="destructive"
              onClick={() => setRemoving(true)}
            >
              <TrashIcon />
              {m['action.remove']()}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <ConfirmDialog
          open={removing}
          onOpenChange={setRemoving}
          title={m['content.remove_title']()}
          description={m['content.remove_description']({ name: item.title })}
          destructive
          confirmLabel={m['action.remove']()}
          onConfirm={() => {
            setRemoving(false);
            handlers.onRemove(item);
          }}
        />
      </div>
    </div>
  );
}
