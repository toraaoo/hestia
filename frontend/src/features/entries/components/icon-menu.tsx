import { ImageIcon, PencilSimpleIcon, TrashIcon } from '@phosphor-icons/react';
import { toast } from 'sonner';

import { dialog } from '@/api';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { m } from '@/paraglide/messages.js';
import {
  useEntryIcons,
  useRemoveEntryIcon,
  useSetEntryIcon,
} from '@/queries/icons';

/**
 * The hero-icon overlay menu: pick a custom image for the entry or reset it
 * to the kind glyph. Desktop-local (the shell's `icons_*` commands).
 */
export function EntryIconMenu({ id }: { id: string }) {
  const icons = useEntryIcons();
  const set = useSetEntryIcon();
  const remove = useRemoveEntryIcon();
  const hasIcon = !!icons.data?.[id];

  const change = async () => {
    const path = await dialog.pickImage();
    if (!path) return;
    set.mutate(
      { entryId: id, sourcePath: path },
      { onError: (error) => toast.error(error.message) },
    );
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <button
            type="button"
            aria-label={m['entry.change_icon']()}
            className="grid size-5 place-items-center bg-background/80 text-muted-foreground ring-1 ring-border backdrop-blur-xs outline-none hover:text-foreground focus-visible:ring-ring"
          >
            <PencilSimpleIcon className="size-3" />
          </button>
        }
      />
      <DropdownMenuContent align="start" className="w-44">
        <DropdownMenuItem onClick={change} disabled={set.isPending}>
          <ImageIcon />
          {m['entry.change_icon']()}
        </DropdownMenuItem>
        <DropdownMenuItem
          disabled={!hasIcon || remove.isPending}
          onClick={() =>
            remove.mutate(id, {
              onError: (error) => toast.error(error.message),
            })
          }
        >
          <TrashIcon />
          {m['entry.reset_icon']()}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
