import type { ComponentProps } from "react";
import { type VariantProps, cva } from "class-variance-authority";
import { cn } from "@/lib/utils";

const statusDotVariants = cva("shrink-0 rounded-full", {
  variants: {
    size: {
      sm: "size-1.75",
      md: "size-2.25",
    },
  },
  defaultVariants: { size: "md" },
});

/** Live-status indicator: glowing grass when up, dim ink when down. */
function StatusDot({
  on,
  size,
  className,
  ...props
}: ComponentProps<"span"> & VariantProps<typeof statusDotVariants> & { on: boolean }) {
  return (
    <span
      data-slot="status-dot"
      data-on={on}
      className={cn(
        statusDotVariants({ size }),
        on ? "bg-grass-500 shadow-glow-grass" : "bg-ink-500",
        className,
      )}
      {...props}
    />
  );
}

export { StatusDot, statusDotVariants };
