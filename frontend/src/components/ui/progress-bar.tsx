import type { ComponentProps } from "react";
import { type VariantProps, cva } from "class-variance-authority";
import { cn } from "@/lib/utils";

const trackVariants = cva(
  "relative overflow-hidden rounded-xs bg-surface-inset shadow-bevel-inset",
  {
    variants: {
      size: { sm: "h-2", md: "h-3.5", lg: "h-5" },
    },
    defaultVariants: { size: "md" },
  },
);

const fillVariants = cva(
  "absolute inset-y-0.5 left-0.5 rounded-[1px] shadow-[inset_0_2px_0_rgb(255_255_255/0.25),inset_0_-2px_0_rgb(0_0_0/0.3)] transition-[width] duration-260 ease-snap",
  {
    variants: {
      tone: { hearth: "bg-hearth-500", success: "bg-grass-500", danger: "bg-tnt-500" },
    },
    defaultVariants: { tone: "hearth" },
  },
);

export type ProgressTone = NonNullable<VariantProps<typeof fillVariants>["tone"]>;
export type ProgressSize = NonNullable<VariantProps<typeof trackVariants>["size"]>;

interface ProgressBarProps extends Omit<ComponentProps<"div">, "children"> {
  value?: number;
  max?: number;
  label?: string;
  showPct?: boolean;
  tone?: ProgressTone;
  size?: ProgressSize;
  segmented?: boolean;
  indeterminate?: boolean;
}

/** Chunky pixel progress bar for downloads / installs. */
function ProgressBar({
  value = 0,
  max = 100,
  label,
  showPct = true,
  tone,
  size,
  segmented = true,
  indeterminate = false,
  className,
  ...props
}: ProgressBarProps) {
  const pct = indeterminate ? 0 : Math.max(0, Math.min(100, (value / max) * 100));
  return (
    <div
      data-slot="progress-bar"
      className={cn("flex w-full flex-col gap-1.5", className)}
      {...props}
    >
      {(label != null || showPct) && (
        <div className="flex items-baseline justify-between gap-3 font-pixel text-xs tracking-wide text-fg-2 uppercase font-crisp">
          <span>{label}</span>
          {showPct && !indeterminate && <span className="text-fg-1">{Math.round(pct)}%</span>}
        </div>
      )}
      <div
        className={cn(trackVariants({ size }))}
        role="progressbar"
        aria-valuenow={indeterminate ? undefined : Math.round(pct)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div
          className={cn(
            fillVariants({ tone }),
            segmented && !indeterminate && "prog-segmented",
            indeterminate && "prog-indeterminate",
          )}
          style={indeterminate ? undefined : { width: `calc(${pct}% - 4px)` }}
        />
      </div>
    </div>
  );
}

export { ProgressBar };
