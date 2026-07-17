import type { ReactNode } from 'react';

import { Checkbox } from '@/components/ui/checkbox';
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { useFieldContext } from '@/hooks/form-context';

/**
 * The reusable, form-bound field components shared by every `useAppForm` form
 * (wizards and settings alike). Each reads its field off the form context, so
 * a call site is just `<field.TextField label="…" />` — the value, blur, change
 * and zod-driven error wiring lives here, once.
 */

/** Show a field's validation errors only once it has been touched. */
function useErrors(meta: { isTouched: boolean; errors: unknown[] }) {
  const show = meta.isTouched && meta.errors.length > 0;
  return show ? (meta.errors as Array<{ message?: string }>) : undefined;
}

export function TextField({
  label,
  description,
  placeholder,
  type = 'text',
  className,
  inputClassName,
}: {
  label?: ReactNode;
  description?: ReactNode;
  placeholder?: string;
  type?: React.ComponentProps<'input'>['type'];
  className?: string;
  inputClassName?: string;
}) {
  const field = useFieldContext<string>();
  const errors = useErrors(field.state.meta);
  return (
    <Field className={className} data-invalid={errors ? true : undefined}>
      {label && <FieldLabel htmlFor={field.name}>{label}</FieldLabel>}
      <Input
        id={field.name}
        type={type}
        placeholder={placeholder}
        className={inputClassName}
        aria-invalid={errors ? true : undefined}
        value={field.state.value}
        onBlur={field.handleBlur}
        onChange={(e) => field.handleChange(e.target.value)}
      />
      {description && <FieldDescription>{description}</FieldDescription>}
      <FieldError errors={errors} />
    </Field>
  );
}

export function NumberField({
  label,
  description,
  min,
  max,
  className,
  inputClassName,
}: {
  label?: ReactNode;
  description?: ReactNode;
  min?: number;
  max?: number;
  className?: string;
  inputClassName?: string;
}) {
  const field = useFieldContext<number>();
  const errors = useErrors(field.state.meta);
  return (
    <Field className={className} data-invalid={errors ? true : undefined}>
      {label && <FieldLabel htmlFor={field.name}>{label}</FieldLabel>}
      <Input
        id={field.name}
        type="number"
        min={min}
        max={max}
        className={inputClassName}
        aria-invalid={errors ? true : undefined}
        value={Number.isNaN(field.state.value) ? '' : field.state.value}
        onBlur={field.handleBlur}
        onChange={(e) =>
          field.handleChange(
            e.target.value === '' ? Number.NaN : Number(e.target.value),
          )
        }
      />
      {description && <FieldDescription>{description}</FieldDescription>}
      <FieldError errors={errors} />
    </Field>
  );
}

export function SliderField({
  label,
  description,
  min,
  max,
  step,
  className,
  sliderClassName,
  formatValue,
}: {
  label?: ReactNode;
  description?: ReactNode;
  min: number;
  max: number;
  step?: number;
  className?: string;
  sliderClassName?: string;
  formatValue?: (value: number) => ReactNode;
}) {
  const field = useFieldContext<number>();
  return (
    <Field className={className}>
      {label && (
        <FieldLabel htmlFor={field.name}>
          {label}
          {formatValue && <> — {formatValue(field.state.value)}</>}
        </FieldLabel>
      )}
      <Slider
        id={field.name}
        className={sliderClassName}
        min={min}
        max={max}
        step={step}
        value={field.state.value}
        onValueChange={(v) => field.handleChange(Array.isArray(v) ? v[0] : v)}
      />
      {description && <FieldDescription>{description}</FieldDescription>}
    </Field>
  );
}

export function CheckboxField({
  label,
  description,
  className,
}: {
  label: ReactNode;
  description?: ReactNode;
  className?: string;
}) {
  const field = useFieldContext<boolean>();
  return (
    <Field orientation="horizontal" className={className}>
      <Checkbox
        id={field.name}
        checked={field.state.value}
        onCheckedChange={(c) => field.handleChange(c === true)}
      />
      <FieldLabel htmlFor={field.name} className="font-normal">
        {label}
        {description && <FieldDescription>{description}</FieldDescription>}
      </FieldLabel>
    </Field>
  );
}

export function SelectField({
  label,
  description,
  options,
  placeholder,
  className,
  triggerClassName,
}: {
  label?: ReactNode;
  description?: ReactNode;
  options: Array<{ value: string; label: ReactNode; className?: string }>;
  placeholder?: string;
  className?: string;
  triggerClassName?: string;
}) {
  const field = useFieldContext<string>();
  return (
    <Field className={className}>
      {label && <FieldLabel htmlFor={field.name}>{label}</FieldLabel>}
      <Select
        value={field.state.value}
        onValueChange={(v) => {
          // Base UI can emit null on clear; never overwrite with it.
          if (v) field.handleChange(v);
        }}
      >
        <SelectTrigger id={field.name} className={triggerClassName}>
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent align="start" alignItemWithTrigger={false}>
          <SelectGroup>
            {options.map((o) => (
              <SelectItem key={o.value} value={o.value} className={o.className}>
                {o.label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      {description && <FieldDescription>{description}</FieldDescription>}
    </Field>
  );
}
