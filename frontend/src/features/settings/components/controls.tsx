import { PlusIcon, XIcon } from '@phosphor-icons/react';
import { type ReactNode, useState } from 'react';

import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/**
 * The controls every settings tab is built from. A binary is a switch because
 * it lands the moment it moves; a typed value is saved on purpose, since
 * nothing else about a text field says the daemon took it.
 */

/** A binary setting that takes effect the moment it moves. */
export function SwitchRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <Field orientation="horizontal">
      <FieldContent>
        <FieldLabel htmlFor={id} className="font-normal">
          {label}
        </FieldLabel>
        {description && <FieldDescription>{description}</FieldDescription>}
      </FieldContent>
      <Switch
        id={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next) => onChange(next === true)}
      />
    </Field>
  );
}

/**
 * A text setting that commits on purpose: Save appears once the value differs
 * and the write confirms with a toast. `confirm` gates the save behind a
 * dialog, for a value whose change moves the whole daemon.
 */
export function SavedInput({
  id,
  value,
  onSave,
  type,
  mono,
  placeholder,
  confirm,
}: {
  id: string;
  value: string;
  onSave: (value: string) => void;
  type?: 'password';
  mono?: boolean;
  placeholder?: string;
  confirm?: { title: string; description: string };
}) {
  const [draft, setDraft] = useState<string | null>(null);
  const current = draft ?? value;
  const dirty = current.trim() !== value;

  const save = () => {
    setDraft(null);
    onSave(current.trim());
  };

  return (
    <div className="flex max-w-md items-center gap-2">
      <Input
        id={id}
        type={type}
        autoComplete={type === 'password' ? 'off' : undefined}
        className={cn('flex-1', mono && 'font-mono')}
        placeholder={placeholder}
        value={current}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && dirty && !confirm) save();
        }}
      />
      {dirty &&
        (confirm ? (
          <ConfirmDialog
            trigger={
              <Button variant="outline" size="sm">
                {m['app.action.save']()}
              </Button>
            }
            title={confirm.title}
            description={confirm.description}
            confirmLabel={m['app.action.save']()}
            onConfirm={save}
          />
        ) : (
          <Button variant="outline" size="sm" onClick={save}>
            {m['app.action.save']()}
          </Button>
        ))}
    </div>
  );
}

/** A bordered list of free-form entries, each removable, with an add row. */
export function RowList({
  label,
  description,
  placeholder,
  values,
  pending,
  onChange,
}: {
  label: string;
  description?: string;
  placeholder: string;
  values: string[];
  pending?: boolean;
  onChange: (values: string[]) => void;
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      {description && <FieldDescription>{description}</FieldDescription>}
      <div className="divide-y divide-border border border-border">
        {values.length === 0 ? (
          <p className="px-3 py-2 text-xs text-muted-foreground">
            {m['settings.list.empty']()}
          </p>
        ) : (
          values.map((value) => (
            <ValueRow
              key={value}
              value={value}
              pending={pending}
              onRemove={() => onChange(values.filter((v) => v !== value))}
            />
          ))
        )}
        <AddRow
          placeholder={placeholder}
          label={m['content.add']()}
          pending={pending}
          onAdd={(value) => {
            if (!values.includes(value)) onChange([...values, value]);
          }}
        />
      </div>
    </Field>
  );
}

/** One entry of a bordered list, removable, optionally tagged. */
export function ValueRow({
  value,
  badge,
  pending,
  onRemove,
}: {
  value: string;
  badge?: ReactNode;
  pending?: boolean;
  onRemove: () => void;
}) {
  return (
    <div className="flex items-center gap-2 px-3 py-1.5">
      <span className="min-w-0 flex-1 truncate font-mono text-xs">{value}</span>
      {badge}
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label={m['app.action.remove']()}
        disabled={pending}
        onClick={onRemove}
      >
        <XIcon className="size-3.5" />
      </Button>
    </div>
  );
}

/** The last row of a bordered list: type a value, add it. */
export function AddRow({
  placeholder,
  label,
  pending,
  onAdd,
}: {
  placeholder: string;
  label: string;
  pending?: boolean;
  onAdd: (value: string) => void;
}) {
  const [draft, setDraft] = useState('');

  const add = () => {
    const value = draft.trim();
    if (!value) return;
    setDraft('');
    onAdd(value);
  };

  return (
    <div className="flex items-center gap-2 bg-muted/30 px-2 py-1.5">
      <Input
        value={draft}
        placeholder={placeholder}
        className="h-7 flex-1 border-transparent bg-transparent font-mono text-xs shadow-none"
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') add();
        }}
      />
      <Button
        variant="ghost"
        size="xs"
        data-icon="inline-start"
        disabled={pending || draft.trim().length === 0}
        onClick={add}
      >
        <PlusIcon weight="bold" className="size-3.5" />
        {label}
      </Button>
    </div>
  );
}
