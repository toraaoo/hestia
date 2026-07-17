import { MagnifyingGlassIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import type { ContentVersion } from '@/features/content/mock';
import { getProject, projectVersions } from '@/features/content/mock';
import type { InstalledContent } from '@/features/entries/mock';
import { agoLabel } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/**
 * Pick a specific published version of an installed item — the mock stand-in
 * for `content.versions`, presented like the create wizard's version picker
 * (search over a selectable list). The swap itself applies at the next launch.
 */
export function ChangeVersionModal({
  item,
  onOpenChange,
  onPick,
}: {
  /** The installed item being re-pinned, or `null` when closed. */
  item: InstalledContent | null;
  onOpenChange: (open: boolean) => void;
  onPick: (item: InstalledContent, version: ContentVersion) => void;
}) {
  const [search, setSearch] = useState('');
  const [pickedId, setPickedId] = useState<string | null>(null);

  const project = item ? getProject(item.id) : undefined;
  const versions = project ? projectVersions(project) : [];
  const q = search.trim().toLowerCase();
  const shown = versions.filter(
    (v) =>
      !q ||
      v.version_number.toLowerCase().includes(q) ||
      v.game_versions.some((g) => g.toLowerCase().includes(q)),
  );
  const picked = versions.find((v) => v.id === pickedId) ?? null;

  const close = (next: boolean) => {
    if (!next) {
      setSearch('');
      setPickedId(null);
    }
    onOpenChange(next);
  };

  return (
    <Dialog open={item !== null} onOpenChange={close}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {m['content.change_version_title']({ name: item?.name ?? '' })}
          </DialogTitle>
          <DialogDescription>
            {m['content.change_version_description']()}
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <div className="relative">
            <MagnifyingGlassIcon className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              className="pl-8"
              placeholder={m['wizard.filter_versions']()}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>

          <div className="max-h-64 divide-y divide-border overflow-y-auto border border-border">
            {shown.length === 0 ? (
              <p className="px-3 py-6 text-center text-xs text-muted-foreground">
                {m['wizard.no_versions_match']()}
              </p>
            ) : (
              shown.map((v) => {
                const current = v.version_number === item?.version;
                const selected = pickedId === v.id;
                return (
                  <button
                    key={v.id}
                    type="button"
                    aria-pressed={selected}
                    onClick={() => setPickedId(v.id)}
                    className={cn(
                      'flex w-full items-center gap-2.5 px-3 py-2 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
                      selected
                        ? 'bg-muted text-foreground'
                        : 'hover:bg-muted/50',
                    )}
                  >
                    <span
                      className={cn(
                        'size-1.5 shrink-0 rounded-full',
                        selected ? 'bg-ember' : 'bg-transparent',
                      )}
                    />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate font-mono text-xs">
                        {v.version_number}
                      </span>
                      <span className="block truncate text-[11px] text-muted-foreground">
                        {v.game_versions.join(', ')} ·{' '}
                        {agoLabel(v.published_unix)}
                      </span>
                    </span>
                    {v.channel !== 'release' && (
                      <Badge variant="outline" className="shrink-0 text-[10px]">
                        {v.channel}
                      </Badge>
                    )}
                    {current && (
                      <Badge
                        variant="secondary"
                        className="shrink-0 text-[10px]"
                      >
                        {m['content.current_version']()}
                      </Badge>
                    )}
                  </button>
                );
              })
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={!picked || picked.version_number === item?.version}
            onClick={() => {
              if (item && picked) onPick(item, picked);
              close(false);
            }}
          >
            {m['action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
