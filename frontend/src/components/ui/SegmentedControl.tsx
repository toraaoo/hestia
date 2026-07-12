import type { ComponentType } from "react";
import { cn } from "../../lib/cn";

interface SegmentedOption<T extends string> {
  value: T;
  title: string;
  icon: ComponentType<{ size?: number }>;
}

interface SegmentedControlProps<T extends string> {
  options: readonly SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
}

/** Compact icon segmented control (view-mode toggles). */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
}: SegmentedControlProps<T>) {
  return (
    <div className="flex gap-0.5 rounded-sm bg-surface-2 p-0.75 shadow-card-flat">
      {options.map(({ value: option, title, icon: OptionIcon }) => (
        <button
          key={option}
          title={title}
          onClick={() => onChange(option)}
          className={cn(
            "flex h-7.5 w-8 items-center justify-center rounded-xs",
            option === value ? "bg-surface-active text-text-1" : "text-text-3 hover:text-text-1",
          )}
        >
          <OptionIcon size={15} />
        </button>
      ))}
    </div>
  );
}
