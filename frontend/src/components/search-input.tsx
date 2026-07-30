import { MagnifyingGlassIcon } from '@phosphor-icons/react';
import { useDebounce } from '@uidotdev/usehooks';
import { useEffect, useRef, useState } from 'react';

import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';

/**
 * A search field: a magnifier-prefixed text input. The one search control every
 * filterable list shares (browse, the install picker, version pickers), so the
 * icon placement and padding stay identical everywhere.
 *
 * The field tracks every keystroke while `onChange` fires only once the typing
 * settles, so a search that costs a request runs once per word rather than once
 * per character. `delay={0}` opts out.
 */
export function SearchInput({
  value,
  onChange,
  placeholder,
  className,
  delay = 300,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  delay?: number;
}) {
  const [draft, setDraft] = useState(value);
  const settled = useDebounce(draft, delay);
  const emitted = useRef(value);

  useEffect(() => {
    if (settled === emitted.current) return;
    emitted.current = settled;
    onChange(settled);
  }, [settled, onChange]);

  useEffect(() => {
    if (value === emitted.current) return;
    emitted.current = value;
    setDraft(value);
  }, [value]);

  return (
    <div className={cn('relative', className)}>
      <MagnifyingGlassIcon className="-translate-y-1/2 absolute top-1/2 left-2.5 size-3.5 text-muted-foreground" />
      <Input
        className="pl-8"
        placeholder={placeholder}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
      />
    </div>
  );
}
