import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

/** Small uppercase group label for side cards and filter groups. */
export function Overline({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <span className={cn("text-xs font-bold tracking-wider text-text-3 uppercase", className)}>
      {children}
    </span>
  );
}
