import type { ReactNode } from 'react';
import type { ContentKind } from '@/api';
import { chipClass } from '@/components/chip';
import { kindInfo } from '@/features/content/lib/kinds';
import { m } from '@/paraglide/messages.js';

/**
 * The kind filter row shared by every content-shaped list (an entry's content
 * tab, the profile pages): an "All" chip, one chip per kind with its count,
 * and an optional trailing action slot.
 */
export function KindChips({
  kinds,
  kind,
  onKindChange,
  count,
  action,
}: {
  kinds: ContentKind[];
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
  count: (kind: ContentKind) => number;
  action?: ReactNode;
}) {
  return (
    <div className="mb-5 flex flex-wrap items-center gap-1.5">
      <button
        type="button"
        className={chipClass(!kind)}
        onClick={() => onKindChange(undefined)}
      >
        {m['label.all']()}
      </button>
      {kinds.map((k) => (
        <button
          key={k}
          type="button"
          className={chipClass(kind === k)}
          onClick={() => onKindChange(k)}
        >
          {kindInfo[k].label()}
          <span className="ml-1.5 font-mono text-[10px] opacity-60">
            {count(k)}
          </span>
        </button>
      ))}
      {action && <div className="ml-auto">{action}</div>}
    </div>
  );
}
