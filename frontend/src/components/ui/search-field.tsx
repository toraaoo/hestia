import { cn } from "@/lib/utils";
import { SearchIcon } from "@/components/icons";

interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  wide?: boolean;
  className?: string;
}

function SearchField({ value, onChange, placeholder, wide = false, className }: SearchFieldProps) {
  return (
    <label
      data-slot="search-field"
      className={cn(
        "flex h-9 items-center gap-2 rounded-sm bg-surface-2 px-3 shadow-card-flat",
        wide ? "w-90" : "w-64",
        className,
      )}
    >
      <SearchIcon size={15} className="shrink-0 text-text-3" />
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full bg-transparent text-sm text-text-1 outline-none placeholder:text-text-3"
      />
    </label>
  );
}

export { SearchField };
