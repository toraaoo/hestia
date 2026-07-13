import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";

interface StatProps extends ComponentProps<"div"> {
  value: ReactNode;
  label: string;
  size?: "md" | "lg";
  accent?: boolean;
}

/** Hero-figure stat card (players, TPS, playtime, mod counts). */
function Stat({ value, label, size = "md", accent = false, className, ...props }: StatProps) {
  return (
    <div
      data-slot="stat"
      className={cn(
        "flex flex-col gap-1 rounded-lg bg-surface-2 px-4 py-3 shadow-card-flat",
        className,
      )}
      {...props}
    >
      <span
        className={cn(
          "font-hero font-crisp",
          size === "lg" ? "text-xl" : "text-lg",
          accent ? "text-grass-400" : "text-fg-1",
        )}
      >
        {value}
      </span>
      <span className="text-xs text-fg-3">{label}</span>
    </div>
  );
}

export { Stat };
