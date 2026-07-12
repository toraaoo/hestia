import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

interface StatProps {
  value: ReactNode;
  label: string;
  size?: "md" | "lg";
  accent?: boolean;
  className?: string;
}

/** Hero-figure stat card (players, TPS, playtime, mod counts). */
export function Stat({ value, label, size = "md", accent = false, className = "" }: StatProps) {
  return (
    <div
      className={cn(
        "flex flex-col gap-1 rounded-lg bg-surface-2 px-4 py-3 shadow-card-flat",
        className,
      )}
    >
      <span
        className={cn(
          "font-hero font-crisp",
          size === "lg" ? "text-xl" : "text-lg",
          accent ? "text-grass-400" : "text-text-1",
        )}
      >
        {value}
      </span>
      <span className="text-xs text-text-3">{label}</span>
    </div>
  );
}
