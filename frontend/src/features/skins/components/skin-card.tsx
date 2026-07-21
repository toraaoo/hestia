import {
  CheckIcon,
  DotsThreeIcon,
  PencilSimpleIcon,
  TrashIcon,
} from '@phosphor-icons/react';

import type { Skin } from '@/api';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { SkinPose } from '@/features/skins/components/render';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/** An external (Mojang-set, never saved) skin arrives unnamed. */
export function skinDisplayName(skin: Skin): string {
  return skin.name || m['skins.unnamed']();
}

export function skinVariantLabel(skin: Skin): string {
  return skin.variant === 'slim' ? m['skins.slim']() : m['skins.wide']();
}

export function SkinGrid({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(7.25rem,1fr))] gap-3">
      {children}
    </div>
  );
}

export function SkinCard({
  skin,
  selected,
  equipped,
  onSelect,
  onEquip,
  onEdit,
  onRemove,
}: {
  skin: Skin;
  selected: boolean;
  equipped: boolean;
  onSelect: () => void;
  onEquip: () => void;
  onEdit?: () => void;
  onRemove?: () => void;
}) {
  return (
    <div
      className={cn(
        'group relative border transition-colors',
        equipped
          ? 'border-ember'
          : selected
            ? 'border-foreground/30 hover:border-ember/40'
            : 'border-border hover:border-ember/40',
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        aria-pressed={selected}
        className="block w-full outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset"
      >
        <div className="flex h-28 items-center justify-center bg-muted/40 pt-2">
          <SkinPose
            texture={skin.texture}
            variant={skin.variant}
            className="h-24 w-full"
          />
        </div>
        <div className="border-t border-border p-2 text-left">
          <div className="truncate text-xs font-medium">
            {skinDisplayName(skin)}
          </div>
          <div className="mt-0.5 font-mono text-[10px] text-muted-foreground">
            {skinVariantLabel(skin)}
          </div>
        </div>
      </button>

      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="secondary"
              size="icon-sm"
              aria-label={m['skins.actions']()}
              className="absolute top-1.5 right-1.5 bg-background/80 opacity-0 backdrop-blur-xs transition-opacity group-hover:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
            >
              <DotsThreeIcon weight="bold" className="size-3.5" />
            </Button>
          }
        />
        <DropdownMenuContent align="start">
          <DropdownMenuItem disabled={equipped} onClick={onEquip}>
            <CheckIcon />
            {equipped ? m['skins.equipped']() : m['action.equip']()}
          </DropdownMenuItem>
          {onEdit && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={onEdit}>
                <PencilSimpleIcon />
                {m['action.edit']()}
              </DropdownMenuItem>
              {onRemove && (
                <DropdownMenuItem variant="destructive" onClick={onRemove}>
                  <TrashIcon />
                  {m['action.delete']()}
                </DropdownMenuItem>
              )}
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
