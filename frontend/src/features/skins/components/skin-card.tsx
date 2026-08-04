import {
  CheckIcon,
  DotsThreeIcon,
  PencilSimpleIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { motion } from 'motion/react';
import { Children } from 'react';

import type { Skin } from '@/api';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { SkinPose } from '@/features/skins/components';
import { listContainer, listItem } from '@/lib/motion';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

export function skinDisplayName(skin: Skin): string {
  if (skin.name) return skin.name;
  if (skin.source === 'external') return m['skin.current']();
  return m['skin.unnamed']();
}

export function skinVariantLabel(skin: Skin): string {
  return skin.variant === 'slim' ? m['skin.slim']() : m['skin.wide']();
}

export function SkinGrid({ children }: { children: React.ReactNode }) {
  return (
    <motion.div
      initial="hidden"
      animate="show"
      variants={listContainer(Children.count(children))}
      className="grid grid-cols-[repeat(auto-fill,minmax(7.25rem,1fr))] gap-3"
    >
      {children}
    </motion.div>
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
    <motion.div
      variants={listItem}
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
        <div className="relative aspect-[31/34] bg-muted/40">
          <SkinPose
            texture={skin.texture}
            variant={skin.variant}
            className="absolute inset-0 size-full"
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
              aria-label={m['skin.actions']()}
              className="absolute top-1.5 right-1.5 bg-background/80 opacity-0 backdrop-blur-xs transition-opacity group-hover:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
            >
              <DotsThreeIcon weight="bold" className="size-3.5" />
            </Button>
          }
        />
        <DropdownMenuContent align="start">
          <DropdownMenuItem disabled={equipped} onClick={onEquip}>
            <CheckIcon />
            {equipped ? m['skin.equipped']() : m['app.action.equip']()}
          </DropdownMenuItem>
          {onEdit && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={onEdit}>
                <PencilSimpleIcon />
                {m['app.action.edit']()}
              </DropdownMenuItem>
              {onRemove && (
                <DropdownMenuItem variant="destructive" onClick={onRemove}>
                  <TrashIcon />
                  {m['app.action.delete']()}
                </DropdownMenuItem>
              )}
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </motion.div>
  );
}
