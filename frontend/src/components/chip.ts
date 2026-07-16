import { cn } from '@/lib/utils';

/** A small filter chip (link or button), highlighted when active. */
export const chipClass = (active: boolean) =>
  cn(
    'border px-2.5 py-1 text-xs transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring',
    active
      ? 'border-transparent bg-primary text-primary-foreground'
      : 'border-border text-muted-foreground hover:bg-muted hover:text-foreground',
  );
