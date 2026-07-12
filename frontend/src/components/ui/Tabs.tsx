import { cn } from "../../lib/cn";

export interface TabItem<T extends string> {
  id: T;
  label: string;
  count?: number;
}

interface TabsProps<T extends string> {
  items: readonly TabItem<T>[];
  value: T;
  onChange: (id: T) => void;
  className?: string;
}

/** Horizontal underline tab bar for top-of-view sub-navigation. */
export function Tabs<T extends string>({ items, value, onChange, className = "" }: TabsProps<T>) {
  return (
    <div
      role="tablist"
      className={cn("flex items-center gap-0.5 border-b border-border-2", className)}
    >
      {items.map(({ id, label, count }) => (
        <button
          key={id}
          role="tab"
          aria-selected={id === value}
          onClick={() => onChange(id)}
          className={cn(
            "relative h-10 px-3.5 text-sm font-semibold transition-colors duration-100",
            id === value ? "text-text-1" : "text-text-3 hover:text-text-1",
          )}
        >
          {label}
          {count != null && <span className="ml-1.5 text-xs font-medium text-text-3">{count}</span>}
          {id === value && (
            <span className="absolute inset-x-2 -bottom-px h-0.75 rounded-t-xs bg-hearth-500" />
          )}
        </button>
      ))}
    </div>
  );
}
