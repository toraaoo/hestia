import type { ReactNode } from 'react';

/** A dashed empty-state placeholder for lists and tabs with nothing to show. */
export function Empty({ children }: { children: ReactNode }) {
  return (
    <p className="border border-dashed border-border px-4 py-10 text-center text-xs text-muted-foreground">
      {children}
    </p>
  );
}
