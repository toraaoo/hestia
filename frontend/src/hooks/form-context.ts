import { createFormHookContexts } from '@tanstack/react-form';

/**
 * The field/form contexts shared by every `useAppForm` form and its bound
 * field components. Kept separate from `hooks/form.tsx` so the reusable field
 * components can read the field context without importing the hook (which
 * imports them — the split breaks that cycle).
 */
export const { fieldContext, formContext, useFieldContext, useFormContext } =
  createFormHookContexts();
