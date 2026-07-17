import { createFormHook } from '@tanstack/react-form';

import {
  CheckboxField,
  NumberField,
  SelectField,
  SliderField,
  TextField,
} from '@/components/form/fields';
import { SubmitButton } from '@/components/form/submit-button';
import { fieldContext, formContext } from '@/hooks/form-context';

/**
 * The app's form hook. `useAppForm` binds the reusable field components
 * (`field.TextField`, `field.SelectField`, …) and the `SubmitButton` form
 * component to a form; `withForm` shares options/props into a subform. Every
 * form in the UI is built through this so validation (zod via `validators`)
 * and field wiring stay uniform. See TanStack Form's multi-step wizard guide
 * for the `FormGroup`-per-step pattern the create/install wizards follow.
 */
export const { useAppForm, withForm } = createFormHook({
  fieldComponents: {
    TextField,
    NumberField,
    SliderField,
    SelectField,
    CheckboxField,
  },
  formComponents: {
    SubmitButton,
  },
  fieldContext,
  formContext,
});
