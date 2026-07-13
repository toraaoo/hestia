import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

/** Small uppercase group label for side cards and filter groups. */
function Overline({ className, ...props }: ComponentProps<"span">) {
  return (
    <span
      data-slot="overline"
      className={cn("text-xs font-bold tracking-wider text-fg-3 uppercase", className)}
      {...props}
    />
  );
}

export { Overline };
