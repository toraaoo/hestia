import { useState } from 'react';

import type { ContentProfile, InstalledContent } from '@/api';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { PickRow } from '@/features/content/pick-row';
import { m } from '@/paraglide/messages.js';

export function MembersDialog({
  profile,
  pool,
  pending,
  onOpenChange,
  onSave,
}: {
  profile: ContentProfile | null;
  pool: InstalledContent[];
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onSave: (name: string, members: string[]) => void;
}) {
  const [selected, setSelected] = useState<string[] | null>(null);
  const members = selected ?? profile?.members ?? [];

  const close = (next: boolean) => {
    if (!next) setSelected(null);
    onOpenChange(next);
  };

  return (
    <Dialog open={profile !== null} onOpenChange={close}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {m['profiles.members_title']({ name: profile?.name ?? '' })}
          </DialogTitle>
          <DialogDescription>
            {m['profiles.members_description']()}
          </DialogDescription>
        </DialogHeader>
        {pool.length === 0 ? (
          <Empty>{m['profiles.members_empty']()}</Empty>
        ) : (
          <div className="grid max-h-72 gap-2 overflow-y-auto p-1">
            {pool.map((item) => {
              const checked = members.includes(item.filename);
              return (
                <PickRow
                  key={item.filename}
                  icon={contentIcon(item.kind)}
                  title={item.title || item.filename}
                  subtitle={`${contentKindLabel[item.kind]()} · ${item.versionNumber || item.filename}`}
                  selected={checked}
                  onSelect={() =>
                    setSelected(
                      checked
                        ? members.filter((f) => f !== item.filename)
                        : [...members, item.filename],
                    )
                  }
                />
              );
            })}
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={pending}
            onClick={() => {
              if (profile) onSave(profile.name, members);
            }}
          >
            {m['action.apply']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
