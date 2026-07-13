import type { ComponentType } from "react";
import { cn } from "@/lib/utils";

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
function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
}: SegmentedControlProps<T>) {
  return (
    <div
      data-slot="segmented-control"
      className="flex gap-0.5 rounded-sm bg-surface-2 p-0.75 shadow-card-flat"
    >
      {options.map(({ value: option, title, icon: OptionIcon }) => (
        <button
          key={option}
          type="button"
          data-slot="segmented-control-item"
          data-active={option === value}
          title={title}
          onClick={() => onChange(option)}
          className={cn(
            "flex h-7.5 w-8 items-center justify-center rounded-xs outline-hidden",
            option === value ? "bg-surface-active text-fg-1" : "text-fg-3 hover:text-fg-1",
          )}
        >
          <OptionIcon size={15} />
        </button>
      ))}
    </div>
  );
}

export { SegmentedControl };
