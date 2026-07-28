import { PlusIcon, XIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { chipClass } from '@/components/chip';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { type Locale, useLocale } from '@/hooks/locale';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { locales } from '@/paraglide/runtime.js';

/** Endonyms — a language always names itself, whatever locale is active. */
const LANGUAGE_NAMES: Record<string, string> = {
  en: 'English',
  'pt-BR': 'Português (Brasil)',
};

export function LanguageField() {
  const { locale, changeLocale } = useLocale();
  return (
    <Field>
      <FieldLabel htmlFor="language">{m['settings.language']()}</FieldLabel>
      <Select
        value={locale}
        onValueChange={(value) => {
          if (value) changeLocale(value as Locale);
        }}
      >
        <SelectTrigger id="language" className="w-full">
          <SelectValue>
            {(value: string) => LANGUAGE_NAMES[value] ?? value}
          </SelectValue>
        </SelectTrigger>
        <SelectContent align="start" alignItemWithTrigger={false}>
          <SelectGroup>
            {locales.map((l) => (
              <SelectItem key={l} value={l}>
                {LANGUAGE_NAMES[l] ?? l}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </Field>
  );
}

export function CheckboxRow({
  id,
  label,
  checked,
  onChange,
  disabled,
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <Field orientation="horizontal">
      <Checkbox
        id={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(c) => onChange(c === true)}
      />
      <FieldLabel htmlFor={id} className="font-normal">
        {label}
      </FieldLabel>
    </Field>
  );
}

/** An editable set of short strings as removable chips with an inline add. */
export function TargetList({
  label,
  placeholder,
  values,
  pending,
  onChange,
}: {
  label: string;
  placeholder: string;
  values: string[];
  pending: boolean;
  onChange: (values: string[]) => void;
}) {
  const [draft, setDraft] = useState('');

  const add = () => {
    const value = draft.trim();
    if (!value || values.includes(value)) return;
    onChange([...values, value]);
    setDraft('');
  };

  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <div className="flex flex-wrap items-center gap-1.5">
        {values.map((value) => (
          <button
            key={value}
            type="button"
            disabled={pending}
            className={cn(chipClass(true), 'flex items-center gap-1')}
            onClick={() => onChange(values.filter((v) => v !== value))}
          >
            <span className="font-mono">{value}</span>
            <XIcon weight="bold" className="size-3 shrink-0" />
          </button>
        ))}
        <div className="flex items-center gap-1">
          <Input
            value={draft}
            placeholder={placeholder}
            className="h-7 w-40 font-mono text-xs"
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') add();
            }}
          />
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={m['content.add']()}
            disabled={pending || draft.trim().length === 0}
            onClick={add}
          >
            <PlusIcon weight="bold" className="size-3.5" />
          </Button>
        </div>
      </div>
    </Field>
  );
}
