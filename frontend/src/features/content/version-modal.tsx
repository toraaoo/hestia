import { MagnifyingGlassIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import type { ContentVersion, InstalledContent } from '@/api';
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
import { agoLabel } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { useContentVersions } from '@/queries/content';

/**
 * Pick a specific published version of an installed item — real
 * `content.versions`, presented like the create wizard's version picker
 * (search over a selectable list). The re-pin applies at the next launch.
 */
export function ChangeVersionModal({
  item,
  loader,
  gameVersion,
  onOpenChange,
  onPick,
}: {
  /** The installed item being re-pinned, or `null` when closed. */
  item: InstalledContent | null;
  loader?: string;
  gameVersion?: string;
  onOpenChange: (open: boolean) => void;
  onPick: (item: InstalledContent, version: ContentVersion) => void;
}) {
  const [search, setSearch] = useState('');
  const [pickedId, setPickedId] = useState<string | null>(null);

  const versions = useContentVersions({
    source: item?.source ?? '',
    project: item?.projectId ?? '',
    loader,
    gameVersion,
  });
  const list = item ? (versions.data ?? []) : [];
  const q = search.trim().toLowerCase();
  const shown = list.filter(
    (v) =>
      !q ||
      v.versionNumber.toLowerCase().includes(q) ||
      v.gameVersions.some((g) => g.toLowerCase().includes(q)),
  );
  const picked = list.find((v) => v.id === pickedId) ?? null;

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
            {m['content.change_version_title']({ name: item?.title ?? '' })}
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
            {versions.isPending ? (
              <p className="px-3 py-6 text-center text-xs text-muted-foreground">
                …
              </p>
            ) : shown.length === 0 ? (
              <p className="px-3 py-6 text-center text-xs text-muted-foreground">
                {m['wizard.no_versions_match']()}
              </p>
            ) : (
              shown.map((v) => {
                const current = v.versionNumber === item?.versionNumber;
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
                        {v.versionNumber}
                      </span>
                      <span className="block truncate text-[11px] text-muted-foreground">
                        {v.gameVersions.join(', ')} ·{' '}
                        {agoLabel(Date.parse(v.datePublished) / 1000)}
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
            disabled={!picked || picked.versionNumber === item?.versionNumber}
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
