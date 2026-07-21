import { useState } from 'react';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { m } from '@/paraglide/messages.js';

export function RenameProfileDialog({
  name,
  taken,
  pending,
  onOpenChange,
  onRename,
}: {
  name: string | null;
  taken: string[];
  pending: boolean;
  onOpenChange: (open: boolean) => void;
  onRename: (name: string, next: string) => void;
}) {
  const [value, setValue] = useState('');
  const trimmed = value.trim();
  const invalid =
    trimmed.length === 0 ||
    trimmed.toLowerCase() === 'none' ||
    taken.some(
      (t) =>
        t.toLowerCase() === trimmed.toLowerCase() &&
        t.toLowerCase() !== name?.toLowerCase(),
    );

  const close = (next: boolean) => {
    if (!next) setValue('');
    onOpenChange(next);
  };

  return (
    <Dialog open={name !== null} onOpenChange={close}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{m['profiles.rename_title']()}</DialogTitle>
        </DialogHeader>
        <Field>
          <FieldLabel>{m['profiles.name_label']()}</FieldLabel>
          <Input
            value={value}
            placeholder={name ?? ''}
            onChange={(e) => setValue(e.target.value)}
            autoFocus
          />
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => close(false)}>
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={invalid || pending}
            onClick={() => {
              if (name) onRename(name, trimmed);
            }}
          >
            {m['profiles.rename']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
