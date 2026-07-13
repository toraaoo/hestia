import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";
import { SearchIcon } from "@/components/icons";

interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  wide?: boolean;
  className?: string;
  onSubmit?: () => void;
}

function SearchField({
  value,
  onChange,
  placeholder,
  wide = false,
  className,
  onSubmit,
}: SearchFieldProps) {
  return (
    <form
      role="search"
      data-slot="search-field"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit?.();
      }}
      className={cn("relative", wide ? "w-90" : "w-64", className)}
    >
      <SearchIcon
        size={15}
        className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-fg-3"
      />
      <Input
        type="search"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="h-9 rounded-sm pl-8.5 text-sm"
      />
    </form>
  );
}

export { SearchField };
