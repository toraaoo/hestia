import { Checkbox } from '@/components/ui/checkbox';
import { Field, FieldLabel } from '@/components/ui/field';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { type Locale, useLocale } from '@/hooks/locale';
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
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <Field orientation="horizontal">
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(c) => onChange(c === true)}
      />
      <FieldLabel htmlFor={id} className="font-normal">
        {label}
      </FieldLabel>
    </Field>
  );
}
