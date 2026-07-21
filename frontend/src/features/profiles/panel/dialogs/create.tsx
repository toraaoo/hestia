import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { m } from '@/paraglide/messages.js';

export function CreateProfileDialog({
  open,
  onOpenChange,
  taken,
  pending,
  onCreate,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taken: string[];
  pending: boolean;
  onCreate: (name: string, seedFromPool: boolean) => void;
}) {
  const [name, setName] = useState('');
  const [seed, setSeed] = useState(true);
  const trimmed = name.trim();
  const invalid =
    trimmed.length === 0 ||
    trimmed.toLowerCase() === 'none' ||
    taken.some((t) => t.toLowerCase() === trimmed.toLowerCase());

  const close = (next: boolean) => {
    if (!next) {
      setName('');
      setSeed(true);
    }
    onOpenChange(next);
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{m['profiles.create_title']()}</DialogTitle>
          <DialogDescription>
            {m['profiles.create_description']()}
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          <Field>
            <FieldLabel>{m['profiles.name_label']()}</FieldLabel>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
            />
          </Field>
          <label
            htmlFor="profile-seed"
            className="flex items-start gap-2.5 text-sm"
          >
            <Checkbox
              id="profile-seed"
              checked={seed}
              onCheckedChange={(checked) => setSeed(checked === true)}
              className="mt-0.5"
            />
            <span>
              {m['profiles.seed_label']()}
              <span className="block text-[11px] text-muted-foreground">
                {m['profiles.seed_hint']()}
              </span>
            </span>
          </label>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={invalid || pending}
            onClick={() => onCreate(trimmed, seed)}
          >
            {m['action.confirm']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
