import { chipClass } from '@/components/chip';
import { SearchInput } from '@/components/search-input';
import { cn } from '@/lib/utils';

/** Search field plus optional filter chips, shared by the target/content steps. */
export function FilterBar({
  search,
  onSearch,
  placeholder,
  chips,
}: {
  search: string;
  onSearch: (v: string) => void;
  placeholder: string;
  chips?: {
    label: string;
    active: boolean;
    disabled?: boolean;
    onClick: () => void;
  }[];
}) {
  return (
    <div className="mb-3 flex flex-col gap-2.5">
      <SearchInput
        value={search}
        onChange={onSearch}
        placeholder={placeholder}
      />
      {chips && chips.length > 1 && (
        <div className="flex flex-wrap gap-1.5">
          {chips.map((c) => (
            <button
              key={c.label}
              type="button"
              disabled={c.disabled}
              className={cn(
                chipClass(c.active),
                c.disabled && 'cursor-not-allowed opacity-40',
              )}
              onClick={c.onClick}
            >
              {c.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
