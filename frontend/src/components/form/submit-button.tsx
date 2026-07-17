import type { ReactNode } from 'react';

import { Button } from '@/components/ui/button';
import { useFormContext } from '@/hooks/form-context';

/**
 * A submit button bound to the form context: disabled while the form is
 * submitting. Rendered inside `<form.AppForm>`.
 */
export function SubmitButton({
  children,
  className,
  disabled,
}: {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
}) {
  const form = useFormContext();
  return (
    <form.Subscribe selector={(state) => state.isSubmitting}>
      {(isSubmitting) => (
        <Button
          type="submit"
          className={className}
          disabled={disabled || isSubmitting}
        >
          {children}
        </Button>
      )}
    </form.Subscribe>
  );
}
