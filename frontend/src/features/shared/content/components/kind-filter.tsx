import type { ContentKind } from '@/api';
import type { FilterGroup } from '@/components/filter-menu';
import { kindInfo } from '@/features/shared/content/lib';
import { m } from '@/paraglide/messages.js';

const ALL = 'all';

/**
 * The content-type dimension shared by every content-shaped list (browse, an
 * entry's content tab, the profile pages, the install picker). `count` adds the
 * tally each list already knows; `all` is dropped where a kind is always one of
 * them, as in the install picker.
 */
export function kindGroup({
  kinds,
  kind,
  onKindChange,
  count,
  disabled,
  all = true,
}: {
  kinds: ContentKind[];
  kind?: ContentKind;
  onKindChange: (kind?: ContentKind) => void;
  count?: (kind: ContentKind) => number;
  disabled?: (kind: ContentKind) => boolean;
  all?: boolean;
}): FilterGroup {
  return {
    label: m['app.label.type'](),
    value: kind ?? ALL,
    neutral: all ? ALL : undefined,
    options: [
      ...(all ? [{ value: ALL, label: m['app.label.all']() }] : []),
      ...kinds.map((k) => ({
        value: k,
        disabled: disabled?.(k),
        label: count ? (
          <>
            {kindInfo[k].label()}
            <span className="ml-1.5 font-mono text-[10px] opacity-60">
              {count(k)}
            </span>
          </>
        ) : (
          kindInfo[k].label()
        ),
      })),
    ],
    onChange: (value) =>
      onKindChange(value === ALL ? undefined : (value as ContentKind)),
  };
}
