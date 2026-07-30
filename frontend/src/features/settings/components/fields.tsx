import { PlusIcon, XIcon } from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import { chipClass } from '@/components/chip';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
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
import { contentQueries } from '@/queries/content';

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
        <SelectContent align="start">
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

/**
 * The CurseForge key, plus whether the daemon now counts the source as one it
 * can serve from — the source list is the authority, so a typo shows as still
 * needing a key rather than as a silent success.
 */
export function SourcesField({
  curseforgeKey,
  onCommit,
}: {
  curseforgeKey: string;
  onCommit: (value: string) => void;
}) {
  const sources = useQuery(contentQueries.sources());
  const ready = (sources.data ?? []).some((s) => s.id === 'curseforge');
  return (
    <Field>
      <FieldLabel htmlFor="curseforge-key" className="gap-2">
        {m['settings.curseforge_key']()}
        <Badge variant={ready ? 'secondary' : 'outline'}>
          {ready
            ? m['settings.source_ready']()
            : m['settings.source_needs_key']()}
        </Badge>
      </FieldLabel>
      <Input
        id="curseforge-key"
        type="password"
        autoComplete="off"
        className="font-mono"
        key={curseforgeKey}
        defaultValue={curseforgeKey}
        onKeyDown={(e) => {
          if (e.key === 'Enter') e.currentTarget.blur();
        }}
        onBlur={(e) => {
          const value = e.target.value.trim();
          if (value !== curseforgeKey) onCommit(value);
        }}
      />
      <FieldDescription>{m['settings.curseforge_key_hint']()}</FieldDescription>
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
