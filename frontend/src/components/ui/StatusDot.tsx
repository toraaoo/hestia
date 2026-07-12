import { cn } from "../../lib/cn";

const SIZES = {
  sm: "size-1.75",
  md: "size-2.25",
};

interface StatusDotProps {
  on: boolean;
  size?: keyof typeof SIZES;
  className?: string;
}

/** Live-status indicator: glowing grass when up, dim ink when down. */
export function StatusDot({ on, size = "md", className = "" }: StatusDotProps) {
  return (
    <span
      className={cn(
        "shrink-0 rounded-full",
        SIZES[size],
        on ? "bg-grass-500 shadow-glow-grass" : "bg-ink-500",
        className,
      )}
    />
  );
}
