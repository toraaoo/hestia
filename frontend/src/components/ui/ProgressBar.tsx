export type ProgressTone = "hearth" | "success" | "danger";
export type ProgressSize = "sm" | "md" | "lg";

const FILLS: Record<ProgressTone, string> = {
  hearth: "bg-hearth-500",
  success: "bg-grass-500",
  danger: "bg-tnt-500",
};

const TRACKS: Record<ProgressSize, string> = {
  sm: "h-2",
  md: "h-3.5",
  lg: "h-5",
};

interface ProgressBarProps {
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
export function ProgressBar({
  value = 0,
  max = 100,
  label,
  showPct = true,
  tone = "hearth",
  size = "md",
  segmented = true,
  indeterminate = false,
}: ProgressBarProps) {
  const pct = indeterminate ? 0 : Math.max(0, Math.min(100, (value / max) * 100));
  return (
    <div className="flex w-full flex-col gap-1.5">
      {(label != null || showPct) && (
        <div className="flex items-baseline justify-between gap-3 font-pixel text-xs tracking-wide text-text-2 uppercase font-crisp">
          <span>{label}</span>
          {showPct && !indeterminate && <span className="text-text-1">{Math.round(pct)}%</span>}
        </div>
      )}
      <div
        className={`relative overflow-hidden rounded-xs bg-surface-inset shadow-bevel-inset ${TRACKS[size]}`}
        role="progressbar"
        aria-valuenow={indeterminate ? undefined : Math.round(pct)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div
          className={`absolute inset-y-0.5 left-0.5 rounded-[1px] shadow-[inset_0_2px_0_rgb(255_255_255/0.25),inset_0_-2px_0_rgb(0_0_0/0.3)] transition-[width] duration-260 ease-snap ${FILLS[tone]} ${segmented && !indeterminate ? "prog-segmented" : ""} ${indeterminate ? "prog-indeterminate" : ""}`}
          style={indeterminate ? undefined : { width: `calc(${pct}% - 4px)` }}
        />
      </div>
    </div>
  );
}
